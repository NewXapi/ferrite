//! `probe` —— 主动探测节点存活与延迟
//!
//! 延迟历史复用 meow 自带的 [`ProxyHealth`]（`ProxyAdapter::health()`）：它已有
//! `record_delay(u16)` / `last_delay()` / `delay_history()`（滚动 10 条）/ `alive()`，
//! 且 `record_delay` 会顺带把 `alive` 置为 `delay > 0`。自造一份等于两套真相。
//!
//! **只拨 TCP，不发 HTTP 请求**：TCP 握手成功已足够判活，而发 HTTP 会打到上游渠道
//! 上、消耗额度并污染用量统计。
//!
//! 只有协议节点（走 meow adapter 的那 7 种）能被探测。`Direct` / `Http` / `Socks5`
//! 走 reqwest，没有 `ProxyAdapter`，也就没有 `health()` 可写——这些节点返回
//! `error = Some(...)` 而非假装成功。

use std::time::{Duration, Instant};

use crate::node::ProxyNode;

/// 单节点探测结果。
#[derive(Debug, Clone)]
pub struct ProbeResult {
    /// 被探测节点的 id（与 `ProxyNode::id` 一致）。
    pub node_id: i64,
    /// 拨号往返毫秒；探测失败为 `None`。
    ///
    /// 上界是 `u16::MAX`（约 65 秒）——meow 的 `ProxyHealth::record_delay` 就吃 `u16`。
    /// 超过上界的耗时会被 clamp（能拨通但慢到 65s 的节点，记 65535 与记真实值对
    /// 决策没有区别，都是"该换节点了"）。
    pub delay_ms: Option<u16>,
    /// 失败原因（不支持探测 / 装配失败 / 拨号失败 / 超时），成功为 `None`。
    pub error: Option<String>,
}

impl ProbeResult {
    /// 探测是否成功（拨通并拿到延迟）。
    pub fn is_alive(&self) -> bool {
        self.delay_ms.is_some()
    }
}

/// 探测单个节点：装配 adapter → `dial_tcp` 计时 → 写回 `health().record_delay()`。
///
/// # 参数
/// - `target`：探测目标，`host:port` 形式（如 `api.openai.com:443`）。用渠道自己的
///   上游主机最能反映真实可用性——同一节点到不同上游的连通性可以完全不同。
/// - `timeout`：拨号超时上限。超时按失败计，且**不**写 `record_delay`
///   （写 0 会让 `ProxyHealth` 把节点标成 dead，而"这次超时"不等于"节点已死"，
///   死活判定交给调用方按连续失败次数决定）。
///
/// # 返回
/// 永不 panic：装配失败、协议不支持、拨号失败、超时都落在 `error` 字段里。
/// 探测是运维动作，不该因为一个坏节点中断整轮。
pub async fn probe_node(node: &ProxyNode, target: &str, timeout: Duration) -> ProbeResult {
    let fail = |msg: String| ProbeResult {
        node_id: node.id,
        delay_ms: None,
        error: Some(msg),
    };

    let (host, port) = match split_host_port(target) {
        Some(hp) => hp,
        None => return fail(format!("探测目标 `{target}` 不是 host:port 形式")),
    };

    // Direct/Http/Socks5 没有 ProxyAdapter（走 reqwest），拿不到 health() 写回。
    let Some(adapter) = crate::adapter::adapter_for(node) else {
        return fail(format!(
            "{:?} 节点不走 meow 适配器，无法用 dial_tcp 探测",
            node.scheme
        ));
    };

    let metadata = crate::adapter::tcp_metadata(host, port);
    let started = Instant::now();
    let dialed = tokio::time::timeout(timeout, adapter.dial_tcp(&metadata)).await;

    match dialed {
        Err(_elapsed) => fail(format!("拨号超时（{} ms）", timeout.as_millis())),
        Ok(Err(e)) => fail(format!("拨号失败: {e}")),
        Ok(Ok(_conn)) => {
            // 连接立即 drop：只验证握手能完成，不发任何字节。
            let elapsed = started.elapsed().as_millis();
            // 0 在 ProxyHealth 里表示 dead；能拨通的亚毫秒延迟记 1。
            let delay = elapsed.clamp(1, u128::from(u16::MAX)) as u16;
            adapter.health().record_delay(delay);
            ProbeResult {
                node_id: node.id,
                delay_ms: Some(delay),
                error: None,
            }
        }
    }
}

/// 拆 `host:port`，兼容 IPv6 字面量（`[::1]:443`）。
///
/// 从右侧找最后一个 `:`：IPv6 地址本身含冒号，从左找会把地址切断。
fn split_host_port(target: &str) -> Option<(&str, u16)> {
    let (host, port) = target.rsplit_once(':')?;
    if host.starts_with('[') != host.ends_with(']') {
        return None;
    }
    let host = host.strip_prefix('[').unwrap_or(host);
    let host = host.strip_suffix(']').unwrap_or(host);
    if host.is_empty() {
        return None;
    }
    Some((host, port.parse().ok()?))
}
