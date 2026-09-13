//! `node_cooldowns` / `channel_node_ids` 访问器行为测试（P1-C 双账本桥数据源）。
//!
//! 测的是 forward 侧降级判定所依赖的对外语义，全部走公开 API：
//! - 冷却表只含「当前仍冷却」的条目：feedback(502) 进表、feedback(200) 清出，
//!   forward 用 `全部绑定节点 ∈ 冷却表` 判定直连回落是否为降级，表里混入
//!   已过期条目就会把健康渠道误判成死渠道；
//! - 绑定表区分「绑了节点」与「本就无绑定」——后者直连是预期路径，不是降级。

use gateway_proxy::manager::ProxyManager;
use gateway_proxy::node::{ProxyNode, ProxyScheme};
use gateway_proxy::pool::ProxySnapshot;
use std::time::Instant;

fn node(id: i64, channels: &[&str]) -> ProxyNode {
    ProxyNode {
        id,
        scheme: ProxyScheme::Http,
        host: format!("n{id}.example"),
        port: 8080,
        auth: None,
        opts: None,
        channel_keys: channels.iter().map(|c| c.to_string()).collect(),
        priority: 0,
    }
}

fn manager_with(nodes: Vec<ProxyNode>) -> ProxyManager {
    let mgr = ProxyManager::new();
    mgr.install(ProxySnapshot { nodes });
    mgr
}

/// 测：无失败时冷却表为空；transport 失败后该节点入表且截止时刻在未来。
/// 为什么：这是 forward 判定"节点确实处于冷却"的唯一信号源。
#[test]
fn feedback_cools_node_into_cooldown_table() {
    let mgr = manager_with(vec![node(1, &["ch"])]);
    assert!(mgr.node_cooldowns().is_empty(), "初始无冷却节点");

    mgr.feedback(1, 502, true);
    let cooled = mgr.node_cooldowns();
    assert_eq!(cooled.len(), 1, "502 传输失败后应恰有一个冷却节点");
    assert_eq!(cooled[0].0, 1);
    assert!(
        cooled[0].1 > Instant::now(),
        "截止时刻必须在未来（仍在冷却）"
    );
}

/// 测：成功反馈清除冷却条目（不返回已过期/已解除的节点）。
/// 为什么：节点恢复后若残留在表里，渠道会被 forward 永久误判为降级。
#[test]
fn success_evicts_node_from_cooldown_table() {
    let mgr = manager_with(vec![node(1, &["ch"])]);
    mgr.feedback(1, 502, true);
    mgr.feedback(1, 200, false);
    assert!(mgr.node_cooldowns().is_empty(), "成功后冷却解除，不得残表");
}

/// 测：直连反馈（node_id == 0）不进冷却表。
/// 为什么：0 不是真实节点；混入会让所有无绑定渠道被误判为"绑定节点全冷却"。
#[test]
fn direct_feedback_is_ignored() {
    let mgr = manager_with(vec![]);
    mgr.feedback(0, 502, true);
    assert!(mgr.node_cooldowns().is_empty());
}

/// 测：channel_node_ids 按绑定返回节点 id，未绑定渠道返回空。
/// 为什么：空表 = "本就无绑定"，forward 据此把直连当预期路径而非降级。
#[test]
fn channel_node_ids_reflects_binding() {
    let mgr = manager_with(vec![node(1, &["ch-a"]), node(2, &["ch-a", "ch-b"])]);
    assert_eq!(mgr.channel_node_ids("ch-a"), vec![1, 2]);
    assert_eq!(mgr.channel_node_ids("ch-b"), vec![2]);
    assert!(mgr.channel_node_ids("ch-none").is_empty());
}
