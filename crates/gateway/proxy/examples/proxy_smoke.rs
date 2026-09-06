//! Smoke: ProxyManager 租约 Client 是否真的把流量送进本地 singbox。
//!
//! 前置：本地 singbox mixed 入站 `127.0.0.1:7890` + clash-api `127.0.0.1:9090`。
//!
//! 运行：`cargo run --example proxy_smoke -p gateway-proxy`
//!
//! 真正区分直连 vs 代理：每步先清空 singbox 连接，发请求后查 httpbin.org
//! 是否出现在 singbox 连接里。走代理时会出现；直连时不会。

use gateway_proxy::manager::ProxyManager;
use gateway_proxy::node::ProxyScheme;
use gateway_proxy::pool::{ProxyNode, ProxySnapshot};

const TARGET_HOST: &str = "httpbin.org";

#[tokio::main]
async fn main() {
    // 隔离系统级代理：让 node_id=0 的直连 Client 真正直连，
    // 否则 ALL_PROXY 会让"直连"也经过 singbox，无法区分。
    unsafe {
        for var in [
            "ALL_PROXY",
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "all_proxy",
            "http_proxy",
            "https_proxy",
        ] {
            std::env::remove_var(var);
        }
    }

    let manager = ProxyManager::new();

    println!("=== 1) 无节点：直连（不应经过 singbox）===");
    {
        let lease = manager.acquire("missing");
        assert_eq!(lease.node_id, 0);
        close_all_connections().await;
        fetch_ip(&lease.client).await;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        assert!(
            !singbox_has_host(TARGET_HOST).await,
            "direct request must NOT appear in singbox connections"
        );
        println!("  ✓ 直连未出现在 singbox 连接中");
    }

    println!("=== 2) 有节点：走本地 singbox socks5 ===");
    manager.install(ProxySnapshot {
        nodes: vec![ProxyNode {
            id: 1,
            scheme: ProxyScheme::Socks5,
            host: "127.0.0.1".into(),
            port: 7890,
            auth: None,
            channel_keys: vec!["ch".into()],
            priority: 10,
        }],
    });
    {
        let lease = manager.acquire("ch");
        assert_eq!(lease.node_id, 1, "should pick the proxy node");
        close_all_connections().await;
        let ip = fetch_ip(&lease.client).await;
        println!("  httpbin 返回 origin: {ip}");
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        assert!(
            singbox_has_host(TARGET_HOST).await,
            "proxied request should appear in singbox connections to {TARGET_HOST}"
        );
        println!("  ✓ 代理请求出现在 singbox 连接中（目标 {TARGET_HOST}）");
    }

    println!("=== 3) 反馈 401 不冷却，仍应命中同一节点 ===");
    {
        let lease = manager.acquire("ch");
        manager.feedback(lease.node_id, 401, false);
        drop(lease);
        let lease2 = manager.acquire("ch");
        assert_eq!(lease2.node_id, 1, "401 must not cooldown the proxy");
        drop(lease2);
        println!("  ✓ 401 后仍能选到代理节点");
    }

    println!("=== 4) 反馈 transport_err 冷却后 fallback 直连 ===");
    {
        let lease = manager.acquire("ch");
        manager.feedback(lease.node_id, 502, true);
        drop(lease);
        let lease2 = manager.acquire("ch");
        assert_eq!(
            lease2.node_id, 0,
            "after cooldown should fall back to direct"
        );
        println!("  ✓ 冷却后直连");
    }

    println!("\nALL SMOKE CHECKS PASSED");
}

async fn fetch_ip(client: &reqwest::Client) -> String {
    let resp = client
        .get("https://httpbin.org/ip")
        .send()
        .await
        .expect("request failed");
    let json: serde_json::Value = resp.json().await.expect("json parse failed");
    json["origin"].as_str().unwrap_or("unknown").to_string()
}

const CLASH_API: &str = "http://127.0.0.1:9090";
const CLASH_SECRET: &str = "e5cfaa2751564a84c4c7aa1481e28870";

async fn close_all_connections() {
    let _ = reqwest::Client::new()
        .delete(format!("{CLASH_API}/connections"))
        .header("Authorization", format!("Bearer {CLASH_SECRET}"))
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await;
}

async fn singbox_has_host(host: &str) -> bool {
    let Ok(resp) = reqwest::Client::new()
        .get(format!("{CLASH_API}/connections"))
        .header("Authorization", format!("Bearer {CLASH_SECRET}"))
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
    else {
        return false;
    };
    let Ok(json) = resp.json::<serde_json::Value>().await else {
        return false;
    };
    json.get("connections")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| c.get("metadata"))
                .filter_map(|m| m.get("host"))
                .filter_map(|h| h.as_str())
                .any(|h| h.contains(host))
        })
        .unwrap_or(false)
}
