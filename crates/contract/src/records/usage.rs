//! 用量记录 — 计量事件与健康观测 (edge 产生, 汇聚 center)。

use super::SyncMeta;
use serde::{Deserialize, Serialize};

/// 一次请求的计量事件 (edge 产生, 汇聚回 center)。
///
/// 这是 edge→center 数据流的核心载荷; `meta.key` 全局唯一, center 以它做幂等去重。
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
    /// 首 token 延迟 (流式)。
    pub first_token_ms: u32,
    pub duration_ms: u32,
    /// 本次调用计费额 (内部单位, 与 UserRecord::quota 同量纲)。
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
    /// 观测到的错误类别, 取值见 contract::error::ErrorCode 体系。
    pub outcome: String,
    pub latency_ms: u32,
    pub observed_at: chrono::DateTime<chrono::Utc>,
}

// TODO(#213): QualityBucket — 按时间桶聚合的质量统计 (延迟分位/成功率), 在
// 03-c 数据聚合路线图启动时再定字段, 避免提前编造。表结构见 07 文档 perf_metrics。
