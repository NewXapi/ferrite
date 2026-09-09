//! 出口接线测试：`[[proxy_nodes]]` 与 `[egress]` 必须真的到达运行期组件。
//!
//! 为什么需要：节点配了但没 `install` 到 `ProxyManager`，模型请求会静默直连；
//! vless 之类的 scheme 混进快照会走 `ProxyClient::Connector`，forward 层直接 502。
//! 这两种失败都不会在解析阶段报错，只能靠行为断言抓。

use gateway::config::{GatewayConfig, build_proxy_snapshot};
use gateway_proxy::{ProxyManager, ProxyScheme};
use std::io::Write;

/// 把 TOML 写到临时文件再 load，走真实的 `GatewayConfig::load` 路径。
fn load_toml(body: &str) -> GatewayConfig {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("ferrite-egress-{}-{n}.toml", std::process::id()));
    let mut f = std::fs::File::create(&path).expect("create temp config");
    f.write_all(body.as_bytes()).expect("write temp config");
    let cfg = GatewayConfig::load(&path).expect("load config");
    let _ = std::fs::remove_file(&path);
    cfg
}

/// `[[proxy_nodes]]` 必须真的进入 `ProxyManager`：绑定的渠道命中该节点、
/// 拿到 reqwest 路径的 Client；未绑定的渠道仍回落直连（`node_id = 0`）。
#[test]
fn proxy_nodes_drive_manager_acquire() {
    let cfg = load_toml(
        r#"
[[proxy_nodes]]
url = "socks5://127.0.0.1:7890"
channel_keys = ["openai"]
priority = 10
"#,
    );
    let snap = build_proxy_snapshot(&cfg.proxy_nodes);
    assert_eq!(snap.nodes.len(), 1);
    assert_eq!(snap.nodes[0].scheme, ProxyScheme::Socks5);
    assert_eq!(snap.nodes[0].host, "127.0.0.1");
    assert_eq!(snap.nodes[0].port, 7890);
    // id 缺省按配置顺序从 1 起编号：0 会与 `Lease` 的直连哨兵值撞车，
    // feedback 会把直连当成节点去冷却。
    assert_eq!(snap.nodes[0].id, 1);
    assert_eq!(snap.nodes[0].priority, 10);

    let manager = ProxyManager::new();
    manager.install(snap);

    let lease = manager.acquire("openai");
    assert_eq!(lease.node_id, 1, "绑定的渠道应选中该节点");
    assert!(
        lease.reqwest_client().is_some(),
        "SOCKS5 必须落在 reqwest 路径；Connector 变体会让 forward 502"
    );

    let miss = manager.acquire("unbound");
    assert_eq!(miss.node_id, 0, "未绑定渠道回落直连");
}

/// 协议节点（vless）现在必须进池并落 adapter 路径；只有「缺 `channel_keys`」
/// 与「非法 URL」两类坏节点该被跳过。
#[test]
fn proxy_nodes_keep_protocol_and_skip_invalid() {
    let cfg = load_toml(
        r#"
[[proxy_nodes]]
url = "vless://11111111-2222-3333-4444-555555555555@example.com:443"
channel_keys = ["openai"]

[[proxy_nodes]]
url = "socks5://127.0.0.1:7890"

[[proxy_nodes]]
url = "not-a-url"
channel_keys = ["openai"]
"#,
    );
    let snap = build_proxy_snapshot(&cfg.proxy_nodes);
    assert_eq!(
        snap.nodes.len(),
        1,
        "只有 vless 该留下，实际 {:?}",
        snap.nodes.iter().map(|n| n.scheme).collect::<Vec<_>>()
    );
    assert_eq!(snap.nodes[0].scheme, ProxyScheme::Vless);

    // vless 必须走 adapter 路径：拿到 adapter 而不是 reqwest client，
    // 否则 forward 会当直连发出去，等于绕过了代理。
    let manager = ProxyManager::new();
    manager.install(snap);
    let lease = manager.acquire("openai");
    assert_eq!(lease.node_id, 1);
    assert!(lease.adapter().is_some(), "vless 必须落 adapter 路径");
    assert!(lease.reqwest_client().is_none(), "vless 不该走 reqwest");
}

/// 显式 `id` 覆盖顺序编号；多节点各自保留自己的 id 与优先级。
#[test]
fn proxy_nodes_explicit_id_wins() {
    let cfg = load_toml(
        r#"
[[proxy_nodes]]
id = 42
url = "http://127.0.0.1:8080"
channel_keys = ["openai"]

[[proxy_nodes]]
url = "socks5://127.0.0.1:7890"
channel_keys = ["openai"]
priority = 5
"#,
    );
    let snap = build_proxy_snapshot(&cfg.proxy_nodes);
    assert_eq!(snap.nodes.len(), 2);
    assert_eq!(snap.nodes[0].id, 42, "显式 id 必须原样保留");
    assert_eq!(snap.nodes[1].id, 2, "缺省 id 用配置下标 + 1");

    // priority 更高的节点优先：socks5 是 5，http 是 0。
    let manager = ProxyManager::new();
    manager.install(snap);
    let lease = manager.acquire("openai");
    assert_eq!(lease.node_id, 2, "高 priority 节点优先");
}
