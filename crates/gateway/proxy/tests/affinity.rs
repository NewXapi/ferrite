//! `ProxyManager::acquire` 软亲和选点行为测试
//!
//! 每个测试验证一个明确的行为契约，注释说明测什么、为什么。
//! 并列打散依赖 `rand::thread_rng`（不可注入种子），因此断言只依赖
//! 亲和语义本身：指针有效时必须粘住；不粘时对具体 id 只做集合断言。

use gateway_proxy::manager::ProxyManager;
use gateway_proxy::node::{ProxyNode, ProxyScheme};
use gateway_proxy::pool::ProxySnapshot;

/// 构造 Http 测试节点（reqwest 原生路径，不触发协议适配器拨号）。
fn node(id: i64, channels: &[&str], priority: i32) -> ProxyNode {
    ProxyNode {
        id,
        scheme: ProxyScheme::Http,
        host: format!("n{id}.example"),
        port: 8080,
        auth: None,
        // NodeOpts 改名（#137）后通用传输层选项字段为 opts；Http 节点无传输层选项。
        opts: None,
        channel_keys: channels.iter().map(|c| c.to_string()).collect(),
        priority,
    }
}

/// 同渠道 "ch" 两个等优先级节点：任一时刻（无冷却）并列最闲组 = {1, 2}。
fn two_nodes_channel() -> ProxyManager {
    let mgr = ProxyManager::new();
    mgr.install(ProxySnapshot {
        nodes: vec![node(1, &["ch"], 0), node(2, &["ch"], 0)],
    });
    mgr
}

/// 测：并列最闲时连续 acquire 粘住同一节点，而不是每请求随机打散。
/// 为什么：ws/grpc/TLS 握手成本按连接支付，打散会重复付费——粘住正是 M4 的目的。
#[test]
fn consecutive_acquires_stick_to_one_node() {
    let mgr = two_nodes_channel();
    let first = mgr.acquire("ch");
    let id = first.node_id;
    assert_ne!(id, 0, "有可用节点时不应走直连");
    drop(first);
    for _ in 0..10 {
        let lease = mgr.acquire("ch");
        assert_eq!(lease.node_id, id, "并列未破且 TTL 内必须粘住亲和节点");
        drop(lease);
    }
}

/// 测：亲和节点被 feedback(502) 冷却后落出候选集，acquire 顺延到另一节点。
/// 为什么：冷却的节点不在并列组内 → 亲和失效 → 单点直通接替者（冷却触发 rebalance）。
#[test]
fn cooled_affinity_node_rebalances_to_other() {
    let mgr = two_nodes_channel();
    let first = mgr.acquire("ch");
    let cooled = first.node_id;
    let other = if cooled == 1 { 2 } else { 1 };
    mgr.feedback(cooled, 502, false);
    drop(first);
    for _ in 0..5 {
        let lease = mgr.acquire("ch");
        assert_eq!(lease.node_id, other, "亲和节点冷却后应由唯一未冷却节点接替");
        drop(lease);
    }
}

/// 测：亲和节点负载拉开（持 Lease 使 inflight 高于层内最小值）后自动让位空闲节点。
/// 为什么：contract 的"rebalance 语义免费得到"——busy 节点落出并列组即被接替，
/// 不需要等冷却或 TTL 过期。
#[test]
fn busy_affinity_node_yields_to_idle_node() {
    let mgr = two_nodes_channel();
    let busy_lease = mgr.acquire("ch");
    let busy = busy_lease.node_id;
    let idle = if busy == 1 { 2 } else { 1 };
    for _ in 0..5 {
        let lease = mgr.acquire("ch");
        assert_eq!(
            lease.node_id, idle,
            "亲和节点 inflight 高于最小时必须选空闲节点"
        );
        drop(lease);
    }
    drop(busy_lease);
}

/// 测：亲和节点冷却恢复后不抢回、不死锁、不漏选。
/// 为什么：恢复（feedback(200) 清冷却）后并列组重新含两节点，contract 明确
/// "不要求特定 id"——只断言结果 ∈ {1,2}（不漏选回直连）且后续选择稳定粘住
/// （亲和指针不会翻转抖动）。
#[test]
fn recovered_node_does_not_grab_back_traffic() {
    let mgr = two_nodes_channel();
    let first = mgr.acquire("ch");
    let cooled = first.node_id;
    mgr.feedback(cooled, 502, false);
    drop(first);

    let second = mgr.acquire("ch");
    let taken = second.node_id;
    assert_ne!(taken, cooled, "冷却期间应顺延到另一节点");
    drop(second);

    // 清冷却：旧节点回到并列组。
    mgr.feedback(cooled, 200, false);
    let after = mgr.acquire("ch");
    let picked = after.node_id;
    assert!(
        picked == 1 || picked == 2,
        "恢复后必须从并列节点中选，不回落直连"
    );
    drop(after);
    for _ in 0..5 {
        let lease = mgr.acquire("ch");
        assert_eq!(
            lease.node_id, picked,
            "恢复后的首次选择应被亲和粘住，不来回翻转"
        );
        drop(lease);
    }
}

/// 测：亲和按 channel_key 隔离，渠道 B 的选点不改写渠道 A 的亲和指针。
/// 为什么：亲和表若共享（如按节点而非渠道 keyed），高频渠道会把低频渠道
/// 钉死在同一节点上，A 的负载无法分散。
#[test]
fn affinity_is_isolated_per_channel() {
    let mgr = ProxyManager::new();
    mgr.install(ProxySnapshot {
        nodes: vec![node(1, &["a", "b"], 0), node(2, &["a", "b"], 0)],
    });
    let a_first = mgr.acquire("a").node_id;
    let b_first = mgr.acquire("b").node_id;
    assert!(a_first == 1 || a_first == 2);
    assert!(b_first == 1 || b_first == 2);

    // 高频锤 b，a 的亲和指针不应被动摇。
    for _ in 0..10 {
        let lease = mgr.acquire("b");
        assert_eq!(lease.node_id, b_first, "b 自身应粘住");
        drop(lease);
    }
    assert_eq!(
        mgr.acquire("a").node_id,
        a_first,
        "渠道 B 的选点不得改写渠道 A 的亲和"
    );
}

/// 测：无节点渠道仍返回直连 node_id=0（回归护栏），且亲和表不会把别的渠道
/// 的节点"漏"进来。
#[test]
fn empty_channel_falls_back_to_direct() {
    let mgr = ProxyManager::new();
    for _ in 0..3 {
        assert_eq!(mgr.acquire("missing").node_id, 0, "无节点渠道必须直连");
    }
    // 另一渠道建立亲和记录后，无节点渠道仍直连。
    mgr.install(ProxySnapshot {
        nodes: vec![node(1, &["ch"], 0)],
    });
    assert_eq!(mgr.acquire("ch").node_id, 1);
    assert_eq!(mgr.acquire("missing").node_id, 0);
}
