//! # records — 领域实体 (逻辑 schema)
//!
//! 这里是 center/edge/web 三方共用的**存储无关**记录定义。
//! 物理编码 (PG 行、Fjall key/value) 一律由 store crate 的 codec 完成,
//! 本模块只描述"一条记录在逻辑上长什么样"。
//!
//! ## 同构记录信封
//!
//! 每条需要同步的记录都包一层 [`SyncMeta`]: key / schema 版本 / 逻辑版本 /
//! 来源节点 / 时间。这是 center-edge 增量同步的最小元数据 (见 02-b 路线图 D0)。
//!
//! ## 命名约定
//!
//! - 后缀 `Record` = 可同步的存储实体 (有 SyncMeta);
//! - 后缀 `Dto`  = 面向前端/API 的投影 (在 api.rs, 从 Record 经 From 转换)。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 所有可同步记录共用的元数据信封。
///
/// - `key`: 节点内全局唯一的逻辑主键 (字符串形式, 便于 Fjall 与 PG 统一编码);
/// - `logical_version`: 该记录自身的单调版本, 修改即 +1;
/// - `origin`: 最初创建该记录的节点 (center 下发配置时为 "center")。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncMeta {
    pub key: String,
    pub schema_version: u32,
    pub logical_version: u64,
    pub origin: String,
    pub updated_at: DateTime<Utc>,
}

/// 上游渠道: 一个可被调度使用的外部供应商接入点。
///
/// 对齐来源: new-api `channels` 表 + sub2api `ent/schema/account.go`。
/// 差异: 多把 key 拆成 `keys: Vec<ChannelKey>`, 调度粒度到 key 而不是 channel。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelRecord {
    pub meta: SyncMeta,
    pub name: String,
    /// 供应商协议族: "openai" | "claude" | "gemini" | ... (小写)
    /// TODO(#210): 改成非枚举字符串还是强枚举? 先用字符串 + 白名单校验, 避免加协议就要改契约。
    pub provider_type: String,
    pub base_url: String,
    /// 该渠道可用的一组上游凭据 (key_index 即本 Vec 的下标, dispatch 按 index 选用)。
    pub keys: Vec<ChannelKey>,
    /// 该渠道全局最大并发 (edge 本地 Semaphore 的容量来源)。
    pub max_concurrency: u32,
    /// 1 启用 / 2 手动禁用 / 3 自动熔断 (由健康观测驱动, center 汇总判定)
    pub status: u8,
    /// 该渠道在哪个分组下可见/可用, 与 GroupRecord::id 对应。
    pub groups: Vec<String>,
}

/// 一把上游凭据。加密责任在 store 层, 契约只关心逻辑结构。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelKey {
    /// 渠道内的 key 下标 (稳定, 删除中间 key 后不重排 — 否则路由快照失效)。
    pub index: u32,
    /// 密文或明文, 由 store 层决定; 契约层不解析。
    pub secret: String,
    /// 单 key 独立限速 (RPM), 0 = 不限制。TODO(#211): 确认是否需要 per-key RPM。
    pub rpm_limit: u32,
}

/// 用户分组: 费率倍率 + 可见模型的载体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupRecord {
    pub meta: SyncMeta,
    pub id: String,
    pub display_name: String,
    /// 该组用户计费时的全局倍率 (对齐 new-api group ratio)。
    pub rate_multiplier: f64,
    /// 该组允许访问的公开模型别名白名单; 空 = 全部可见。
    pub allowed_models: Vec<String>,
}

/// 用户。
///
/// 字段对齐 mock::users::User 与 new-api /api/user/ 的并集;
/// `quota` 沿用 new-api 语义 (内部计费最小单位, 500_000 = $1)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRecord {
    pub meta: SyncMeta,
    pub username: String,
    pub display_name: String,
    pub email: String,
    /// 剩余额度 (new-api 内部单位)。
    pub quota: i64,
    /// 已消耗额度。
    pub used_quota: i64,
    pub request_count: u64,
    pub group: String,
    /// 1 启用 / 2 禁用 (对齐 new-api)。
    pub status: u8,
    /// 1 用户 / 10 管理员 / 100 root。
    pub role: u16,
    pub created_at: DateTime<Utc>,
}

/// 前端/客户端调用 API 用的令牌 (sk-...)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenRecord {
    pub meta: SyncMeta,
    pub user_key: String, // 指向 UserRecord::meta.key
    pub name: String,
    /// SHA-256(明文 key)。明文只在创建响应里出现一次, 契约层永不存明文。
    pub key_hash: String,
    pub group: String,
    /// 令牌独立限额; unlimited_quota = true 时忽略。
    pub quota: i64,
    pub unlimited_quota: bool,
    /// 可选过期时间。
    pub expires_at: Option<DateTime<Utc>>,
    pub status: u8,
}

/// 路由调度单元 — dispatch 状态机的最小选择对象。
///
/// 解耦了 new-api "channel 大表混装多模型多 key" 的问题:
/// (渠道, key, 上游模型) 三元组显式化, 公开别名到上游真名的映射在这里。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteUnitRecord {
    pub meta: SyncMeta,
    pub group: String,
    /// 客户端请求的模型名 (公开别名), 如 "gpt-4o"。
    pub public_model: String,
    pub channel_key: String, // 指向 ChannelRecord::meta.key
    /// ChannelRecord::keys 的下标。
    pub key_index: u32,
    /// 实际发往上游的模型名, 如 "gpt-4o-2024-08-06"。
    pub upstream_model: String,
    /// 数字越大越优先; dispatch 先按 priority 分层, 层内按 weight 加权。
    pub priority: i32,
    pub weight: u32,
    pub status: u8,
}

/// 一次请求的计量事件 (edge 产生, 汇聚回 center)。
///
/// 这是 edge→center 数据流的核心载荷; `mutation_id` 全局唯一, center 以它做幂等去重。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageEventRecord {
    /// = SyncMeta.key, 由 edge 生成的 UUIDv7 (时间有序)。
    pub meta: SyncMeta,
    pub token_key: String,
    pub user_key: String,
    pub channel_key: String,
    pub route_unit_key: String,
    pub public_model: String,
    pub upstream_model: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cached_tokens: u64,
    /// 首 token 延迟 (流式) / 总耗时的一部分。
    pub first_token_ms: u32,
    pub duration_ms: u32,
    /// 本次调用计费额 (new-api 内部单位, 与 UserRecord::quota 同量纲)。
    pub cost: i64,
    /// 上游最终状态码; 0 = 连接失败。
    pub status_code: u16,
    /// 失败摘要 (上游错误信息截断), 成功为 None。
    pub error: Option<String>,
}

/// 一次上游健康观测 (edge 本地产生, 可选上报 center 用于全局熔断)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthObservationRecord {
    pub meta: SyncMeta,
    pub channel_key: String,
    /// 观测到的错误类别, 见 dispatch crate 的错误分类。
    /// TODO(#212): 类别枚举放 contract 还是 dispatch? 倾向 dispatch 内部定义 + 这里存字符串。
    pub outcome: String,
    pub latency_ms: u32,
    pub observed_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// TODO(#213): QualityBucket — 按时间桶聚合的质量统计 (延迟分位/成功率), 在
// 03-c 数据聚合路线图启动时再定字段, 避免提前编造。
// ---------------------------------------------------------------------------
