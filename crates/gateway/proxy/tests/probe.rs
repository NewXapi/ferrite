//! `probe` 单元测试骨架
//!
//! 场景登记：每个 `#[test] #[ignore = "TODO(#111): 骨架未实现"]` 函数体用 `todo!()`，
//! 注释写清预期与理由，供实现者在填充实现时参考。

use gateway_proxy::node::ProxyNode;
use gateway_proxy::probe::{ProbeResult, probe_node};
use std::time::Duration;

/// 固定被测符号的引用，让骨架阶段的 import 不触发 unused（实现时删掉本函数）。
async fn _skeleton_symbol_refs(node: &ProxyNode) {
    let _: ProbeResult = probe_node(node, "example.com:443", Duration::from_secs(5)).await;
}

#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn probe_assembly_failure() {
    // 预期：对于装配失败的节点（如 UUID 非法 / cipher 不识别 / adapter_for 返回 None），
    // probe_node 应返回 error 字段非 None，不 panic。日志里应有 warning。
    todo!("TODO(#111): 装配失败返回 error")
}

#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn probe_timeout() {
    // 预期：设置极小 timeout 时，probe_node 返回 delay_ms = None，error 含超时关键字，
    // health().record_delay() 应未被调用（或记录 None）。
    todo!("TODO(#111): 超时被归类为失败")
}

#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn probe_success_records_delay() {
    // 预期：成功探测（adapter.health().record_delay(u16) 成功）应写回 adapter 的
    // last_delay_ms，使 node_stats 能看到 last_delay_ms = Some(>0)。测试需 mock adapter
    // 并验证 record_delay 调用；省略细节后仅断言 probe_node 返回 delay_ms Some(_)。
    todo!("TODO(#111): 成功探测写回 last_delay")
}

#[test]
#[ignore = "TODO(#111): 骨架未实现"]
fn node_stats_reflects_inflight_and_cooldown() {
    // 预期：node_stats 导出 manager 私有字段 inflight / health / cooldown_remaining_secs，
    // 包含节点 id、inflight 计数、failure_count、冷却剩余秒数（0 表示未冷却），
    // 并带 optional last_delay_ms（来自 adapter.health.last_delay()）。
    todo!("TODO(#111): node_stats 反映 inflight 与冷却剩余")
}
