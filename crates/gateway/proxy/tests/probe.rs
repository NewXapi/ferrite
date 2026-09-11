//! `probe` 与 `node_stats` 的行为测试。
//!
//! 成功拨号需要真实代理服务器，本地造不出来（协议握手无法用 TcpListener 模拟），
//! 故按 `forward/tests/protocol_live.rs` 的惯例做 env 门控：给了
//! `FERRITE_PROXY_PROBE_URL` 才跑真拨，否则跳过。其余三条（装配失败、超时、
//! node_stats 导出）都是可确定验证的真实路径。

use std::time::Duration;

use gateway_proxy::node::{BasicAuth, ProxyNode, ProxyScheme};
use gateway_proxy::pool::ProxySnapshot;
use gateway_proxy::probe::probe_node;
use gateway_proxy::{ProxyManager, node};

/// 造一个能被 meow-config 成功装配的 ss 节点，指向给定 host:port。
///
/// ss 的 `auth.user` 是 cipher、`auth.pass` 是密码（见 `adapter.rs::clash_config`）。
fn ss_node(id: i64, host: &str, port: u16) -> ProxyNode {
    ProxyNode {
        id,
        scheme: ProxyScheme::Shadowsocks,
        host: host.into(),
        port,
        auth: Some(BasicAuth {
            user: "aes-128-gcm".into(),
            pass: "probe-test-pass".into(),
        }),
        vless: None,
        channel_keys: vec!["ch".into()],
        priority: 0,
    }
}

/// 走 reqwest 的 scheme 没有 `ProxyAdapter`，也就没有 `health()` 能写回延迟。
///
/// 预期返回 `error` 而不是假装成功：如果这里返回 `delay_ms: Some(0)`，管理台会把
/// 一个从未被探测的节点显示成"延迟 0ms 的最快节点"。
#[tokio::test]
async fn probe_rejects_non_adapter_scheme() {
    let mut n = ss_node(1, "127.0.0.1", 1080);
    n.scheme = ProxyScheme::Socks5;
    let r = probe_node(&n, "127.0.0.1:9", Duration::from_millis(50)).await;
    assert_eq!(r.delay_ms, None);
    assert!(!r.is_alive());
    assert!(
        r.error.as_deref().unwrap_or_default().contains("适配器"),
        "错误信息应说明该 scheme 不走适配器，实际: {:?}",
        r.error
    );
}

/// 装配失败的节点（cipher 不被 meow-config 识别）返回 error，不 panic。
///
/// `adapter_for` 内部对非法配置 warn 后返回 None——探测必须把它当失败上报，
/// 而不是让整轮探测崩掉。
#[tokio::test]
async fn probe_reports_assembly_failure() {
    let mut n = ss_node(2, "127.0.0.1", 8388);
    n.auth = Some(BasicAuth {
        user: "not-a-real-cipher".into(),
        pass: "x".into(),
    });
    let r = probe_node(&n, "127.0.0.1:9", Duration::from_millis(50)).await;
    assert_eq!(r.delay_ms, None);
    assert!(r.error.is_some());
}

/// 非法探测目标（缺端口）在拨号前就被拒，不消耗 timeout。
#[tokio::test]
async fn probe_rejects_malformed_target() {
    let n = ss_node(3, "127.0.0.1", 8388);
    let r = probe_node(&n, "no-port-here", Duration::from_secs(30)).await;
    assert_eq!(r.delay_ms, None);
    assert!(
        r.error.as_deref().unwrap_or_default().contains("host:port"),
        "应指出目标格式问题，实际: {:?}",
        r.error
    );
}

/// 拨不通的地址在 timeout 内被归类为失败。
///
/// 目标用 TEST-NET-1（192.0.2.0/24，RFC 5737 保留给文档用，公网不可路由），
/// 保证连接不会意外成功；1ms timeout 让用例快速收敛。
#[tokio::test]
async fn probe_unreachable_node_fails_within_timeout() {
    let n = ss_node(4, "192.0.2.1", 8388);
    let r = probe_node(&n, "192.0.2.2:443", Duration::from_millis(1)).await;
    assert_eq!(r.delay_ms, None);
    assert!(r.error.is_some());
}

/// `node_stats` 必须反映 `acquire` 造成的 inflight 与 `feedback` 造成的冷却。
///
/// 这两个字段此前完全不可观测（`inflight` / `health` 是私有），管理台看不到任何
/// 运行时状态。用例走公开 API：install → acquire（持 Lease 保持 inflight）→
/// feedback(502) → 读 stats。
#[tokio::test]
async fn node_stats_reports_inflight_and_cooldown() {
    let mgr = ProxyManager::new();
    mgr.install(ProxySnapshot {
        nodes: vec![ss_node(7, "192.0.2.1", 8388)],
    });

    // 未使用时：无 inflight、无冷却、无延迟记录。
    let idle = mgr.node_stats();
    assert_eq!(idle.len(), 1);
    assert_eq!(idle[0].node_id, 7);
    assert_eq!(idle[0].inflight, 0);
    assert_eq!(idle[0].cooldown_remaining_secs, 0);
    assert_eq!(idle[0].last_delay_ms, None);

    // 持有 Lease 期间 inflight 应为 1；Lease drop 后归零。
    {
        let lease = mgr.acquire("ch");
        assert_eq!(lease.node_id, 7, "唯一可用节点应被选中");
        let busy = mgr.node_stats();
        assert_eq!(busy[0].inflight, 1);
    }
    assert_eq!(
        mgr.node_stats()[0].inflight,
        0,
        "Lease drop 后应释放 inflight"
    );

    // 一次 502 触发首轮冷却（30s 起步的指数退避）。
    mgr.feedback(7, 502, false);
    let cooled = mgr.node_stats();
    assert_eq!(cooled[0].failure_count, 1);
    assert!(
        cooled[0].cooldown_remaining_secs > 0,
        "502 应进入冷却，实际剩余 {}s",
        cooled[0].cooldown_remaining_secs
    );

    // 冷却中的节点不该再被选出，acquire 回落直连（node_id = 0）。
    assert_eq!(mgr.acquire("ch").node_id, 0);
}

/// 一个节点绑多个渠道时，`node_stats` 只出现一次（按 id 去重）。
///
/// 索引是 channel_key → 节点列表，绑 3 个渠道就在 3 个桶里；不去重会让管理台
/// 看到三行同一节点，探测也会重复拨三次。
#[tokio::test]
async fn node_stats_dedupes_multi_channel_node() {
    let mgr = ProxyManager::new();
    let mut n = ss_node(9, "192.0.2.1", 8388);
    n.channel_keys = vec!["a".into(), "b".into(), "c".into()];
    mgr.install(ProxySnapshot { nodes: vec![n] });
    assert_eq!(mgr.node_stats().len(), 1);
}

/// 真节点拨号验证：给 `FERRITE_PROXY_PROBE_URL` 才跑。
///
/// 成功路径必须打到真实代理服务端（协议握手无法本地模拟），故 env 门控。
/// 与 `forward/tests/protocol_live.rs` 同一约定。
#[tokio::test]
async fn probe_live_node_records_delay() {
    let Ok(url) = std::env::var("FERRITE_PROXY_PROBE_URL") else {
        eprintln!("跳过：未设置 FERRITE_PROXY_PROBE_URL");
        return;
    };
    let target =
        std::env::var("FERRITE_PROXY_PROBE_TARGET").unwrap_or_else(|_| "example.com:443".into());
    let mut n = node::ProxyNode::parse_url(&url).expect("FERRITE_PROXY_PROBE_URL 应可解析");
    n.id = 1;
    n.channel_keys = vec!["ch".into()];
    let r = probe_node(&n, &target, Duration::from_secs(10)).await;
    assert!(
        r.is_alive(),
        "真节点应拨通 {target}，失败原因: {:?}",
        r.error
    );
    assert!(
        r.delay_ms.unwrap() > 0,
        "延迟应大于 0（0 在 ProxyHealth 里表示 dead）"
    );
}
