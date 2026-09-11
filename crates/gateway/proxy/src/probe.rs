//! `probe` —— 主动探测节点存活与延迟
//!
//! 复用 meow 自带的 `ProxyHealth`（`meow_common::ProxyAdapter::health()` 返回 `&ProxyHealth`，
//! 有 `record_delay(u16)` / `last_delay()` / `delay_history()` 滚动 10 条 / `alive()`）。
//! 不要自己造延迟历史。
//!
//! 只拨 TCP 不发 HTTP 请求：够判活，且不消耗上游渠道额度。`probe_all` 默认关闭（机场流量）。
use crate::node::ProxyNode;
use std::time::Duration;

/// 单节点探测结果。
pub struct ProbeResult {
    pub node_id: i64,
    /// 拨号往返毫秒；探测失败为 None
    pub delay_ms: Option<u16>,
    /// 失败原因（装配失败 / 拨号失败 / 超时），成功为 None
    pub error: Option<String>,
}

/// 探测单个节点：装配 adapter → `dial_tcp` 计时 → 写回 `health().record_delay()`。
///
/// 只拨 TCP 不发 HTTP 请求：够判活，且不消耗上游渠道额度。
/// `target` 是探测目标 host:port（缺省建议渠道 base_url 的 host，实现时定）。
pub async fn probe_node(node: &ProxyNode, target: &str, timeout: Duration) -> ProbeResult {
    let _ = (node, target, timeout);
    todo!("TODO(#111): 实现单节点探测")
}
