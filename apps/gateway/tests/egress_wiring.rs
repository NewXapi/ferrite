//! 出口接线测试：`[[proxy_nodes]]` 与 `[egress]` 必须真的到达运行期组件。
//!
//! 为什么需要：节点配了但没 `install` 到 `ProxyManager`，模型请求会静默直连；
//! vless 之类的 scheme 混进快照会走 `ProxyClient::Connector`，forward 层直接 502。
//! 这两种失败都不会在解析阶段报错，只能靠行为断言抓。

use gateway::config::{EgressConfig, GatewayConfig, build_proxy_snapshot};
use gateway::sidecar::ShoesSidecar;
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

/// 三类坏节点都必须被跳过：vless（走 Connector，forward 尚未桥接）、
/// 缺 `channel_keys`（永远选不中，只会污染快照）、非法 URL。
#[test]
fn proxy_nodes_skip_unsupported_and_invalid() {
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
    assert!(
        snap.nodes.is_empty(),
        "vless / 空 channel_keys / 非法 URL 都应跳过，实际留下 {} 条",
        snap.nodes.len()
    );
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

/// `binary` 为空 = 不起进程。这是单机默认路径，不能被 sidecar 卡住启动。
#[tokio::test]
async fn sidecar_empty_binary_does_not_spawn() {
    let cfg = EgressConfig::default();
    assert!(cfg.binary.is_empty(), "默认不配 shoes");
    let spawned = ShoesSidecar::spawn(&cfg)
        .await
        .expect("binary 为空应是 Ok(None)");
    assert!(spawned.is_none());
}

/// `binary` 有值但 YAML 不存在必须报错。静默直连会让用户以为流量在走代理。
#[tokio::test]
async fn sidecar_missing_config_is_error() {
    let cfg = EgressConfig {
        binary: "/bin/true".into(),
        config: "/nonexistent/shoes.yaml".into(),
        listen: "127.0.0.1:1".into(),
    };
    let err = ShoesSidecar::spawn(&cfg)
        .await
        .expect_err("缺配置文件必须失败");
    assert!(
        err.to_string().contains("shoes config not found"),
        "错误应指明缺配置文件，实际: {err}"
    );
}

/// 子进程提前退出必须立刻报错，而不是干等满超时窗口再说"入站没起来"。
/// `/bin/true` 立即退出，正好复现 shoes 配置错误自己 exit 的场景。
#[tokio::test]
async fn sidecar_detects_early_exit() {
    let dir = std::env::temp_dir().join(format!("ferrite-sidecar-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let yaml = dir.join("shoes.yaml");
    std::fs::write(&yaml, "- address: \"127.0.0.1:0\"\n").expect("write yaml");

    // 端口必须真空闲：让内核分配再立刻释放。写死端口号会撞上机器上已有的
    // 监听者（本机 17890 就被别的进程占着），connect 成功后测试会假过。
    let free_port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
        l.local_addr().expect("local_addr").port()
    };

    let cfg = EgressConfig {
        binary: "/bin/true".into(),
        config: yaml.to_string_lossy().into_owned(),
        listen: format!("127.0.0.1:{free_port}"),
    };
    let started = std::time::Instant::now();
    let err = ShoesSidecar::spawn(&cfg)
        .await
        .expect_err("子进程退出必须失败而不是假装就绪");
    assert!(
        err.to_string().contains("exited before inbound ready"),
        "错误应指明子进程提前退出，实际: {err}"
    );
    // 不能靠 10s 超时兜底：那样配置写错要等十秒才知道。
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "应快速失败，实际耗时 {:?}",
        started.elapsed()
    );
    let _ = std::fs::remove_dir_all(&dir);
}
