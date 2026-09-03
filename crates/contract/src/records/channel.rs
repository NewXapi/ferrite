//! 渠道记录 — 上游供应商接入点。
//!
//! 参考: new-api internal/catalog/store_channel.go (多 key/设置),
//! sub2api ent/schema/account.go (调度元数据), 07-database-schema.md 域一。

use super::SyncMeta;
use serde::{Deserialize, Serialize};

/// 上游渠道: 一个可被调度使用的外部供应商接入点。
///
/// 差异设计: 多把 key 拆成 `keys: Vec<ChannelKey>`, 调度粒度到 key 而不是 channel
/// (new-api 的 channel 大表把 key 串在一列, 我们显式化)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelRecord {
    pub meta: SyncMeta,
    pub name: String,
    /// 供应商协议族: "openai" | "claude" | "gemini" | "passthrough" (小写)。
    /// 与 protocol::Protocol 白名单对应。
    pub provider_type: String,
    pub base_url: String,
    /// 该渠道可用的一组上游凭据 (key_index 即本 Vec 的下标, dispatch 按 index 选用)。
    pub keys: Vec<ChannelKey>,
    /// 该渠道全局最大并发 (edge 本地 Semaphore 的容量来源)。
    pub max_concurrency: u32,
    /// 1 启用 / 2 手动禁用 / 3 自动熔断 (由健康观测驱动, center 汇总判定)
    pub status: u8,
    /// 该渠道在哪些分组下可见/可用, 与 GroupRecord::id 对应。
    pub groups: Vec<String>,
    /// 渠道级覆盖: 超时/请求头覆盖/参数覆盖/系统提示词。
    /// 结构延后 (TODO(#206)), 先留 JSONB 语义。
    pub settings: serde_json::Value,
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
