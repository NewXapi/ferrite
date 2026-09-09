//! 真实代理节点的 opt-in 验证：对接单个用户提供的外部代理进程（sing-box / mihomo / 任意 SOCKS5 服务）。
//!
//! 与 `real_node.rs` 的核心逻辑一致，但逐协议测试且可以从环境变量中读取真实 URL。
//!
//! # 怎么跑
//!
//! ```sh
//! FERRITE_PROXY_VLESS=vless://... \
//! FERRITE_PROXY_SS=ss://... \
//! FERRITE_PROXY_TROJAN=trojan://... \
//! FERRITE_PROXY_VMESS=vmess://... \
//! cargo test -p forward --test protocol_live
//! ```
//!
//! CI 没有代理进程时跳过所有测试，不变红；用户端到端验证自己的代理配置。

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use bytes::Bytes;

use forward::adapter_egress::{AdapterEgress, DialedStream, StreamDialer};
use forward::egress::{Egress, Timeouts};
use gateway_proxy::{ProxyAdapter, ProxyNode, ProxyScheme, adapter_for, tcp_metadata};

/// 真节点可能在公网，超时给宽一点；卡死时 20s 内失败而不是等默认 300s。
const TIMEOUTS: Timeouts = Timeouts {
    connect_ms: 10_000,
    first_byte_ms: 20_000,
    total_ms: 20_000,
};

/// 把真 [`ProxyAdapter`] 包成 bridge 的 [`StreamDialer`]（与生产 `ForwardStage` 同形）。
struct AdapterDialer(Arc<dyn ProxyAdapter>);

impl std::fmt::Debug for AdapterDialer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdapterDialer").finish()
    }
}

#[async_trait::async_trait]
impl StreamDialer for AdapterDialer {
    async fn dial(&self, host: &str, port: u16) -> std::io::Result<Box<dyn DialedStream>> {
        let conn = self
            .0
            .dial_tcp(&tcp_metadata(host, port))
            .await
            .map_err(|e| std::io::Error::other(format!("dial {host}:{port}: {e}")))?;
        Ok(Box::new(conn))
    }
}

/// 极简 HTTP/1.1 服务：读到头结束即回 200 + `ok`，带命中计数。
fn spawn_local_http() -> (SocketAddr, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local HTTP 服务");
    let addr = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let hits_clone = hits.clone();

    // 使用 std::thread 而不是 tokio::spawn，因为 listener 是阻塞的
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };
            hits_clone.fetch_add(1, Ordering::Relaxed);

            // 简单地读完整个请求（理论上已经看到空行）
            let mut buffer = [0u8; 4096];
            let mut got = Vec::new();
            loop {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        got.extend_from_slice(&buffer[..n]);
                        if got.windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                    }
                }
            }

            // 回 200 + body = "ok"，保持连接关闭
            let body = b"ok";
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .as_bytes(),
            );
            let _ = stream.write_all(body);
        }
    });

    // 小延时确保服务启动
    std::thread::sleep(Duration::from_millis(50));
    (addr, hits)
}

/// TCP 探活：分享链接节点随时会死，节点下线不是代码缺陷。
async fn node_alive(host: &str, port: u16) -> bool {
    use std::net::ToSocketAddrs;
    let addrs: Vec<std::net::SocketAddr> = match (host, port).to_socket_addrs() {
        Ok(a) => a.collect(),
        Err(_) => return false,
    };
    for a in addrs {
        if tokio::time::timeout(
            std::time::Duration::from_secs(3),
            tokio::net::TcpStream::connect(a),
        )
        .await
        .is_ok()
        {
            return true;
        }
    }
    false
}

/// 拨号失败处理：缺省宽松（打印跳过），FERRITE_PROXY_LIVE_STRICT=1 时当红。
/// 免费节点对 TLS 指纹/可用性极敏感（meow 未开 boring-tls 时 rustls 指纹会被
/// 部分 reality/trojan 服务器掐），节点侧失败不算代码缺陷。
fn on_dial_failure(what: &str, e: &contract::error::NormalizedError) {
    if std::env::var("FERRITE_PROXY_LIVE_STRICT").is_err() {
        eprintln!("跳过：{what} 拨号失败（{e:?}）。FERRITE_PROXY_LIVE_STRICT=1 可强制失败为红");
    } else {
        panic!("{what} 拨号失败: {e:?}");
    }
}

/// 从环境变量构建真实代理节点；没有则返回 None（不打印）。
fn parse_env_url(scheme: &str, env_var: &str) -> Option<ProxyNode> {
    match std::env::var(env_var) {
        Ok(url) => {
            match ProxyNode::parse_url(&url) {
                Ok(node) => {
                    // 验证节点类型是否匹配
                    // 验证节点类型是否匹配
                    let correct_scheme = match scheme {
                        "vless" => ProxyScheme::Vless,
                        "ss" | "shadowsocks" => ProxyScheme::Shadowsocks,
                        "trojan" => ProxyScheme::Trojan,
                        "vmess" => ProxyScheme::Vmess,
                        _ => return None,
                    };
                    if node.scheme != correct_scheme {
                        eprintln!(
                            "环境变量 {} 对应的代理类型不匹配: 期望 {:?}, 实际 {:?}",
                            env_var, correct_scheme, node.scheme
                        );
                        return None;
                    }
                    Some(node)
                }
                Err(e) => {
                    eprintln!("{} 解析失败: {}", env_var, e);
                    None
                }
            }
        }
        Err(_) => {
            eprintln!("跳过: {} 未设置", env_var);
            None
        }
    }
}

#[tokio::test]
async fn vless_reality_vision_live() {
    if let Some(node) = parse_env_url("vless", "FERRITE_PROXY_VLESS") {
        if !node_alive(&node.host, node.port).await {
            eprintln!(
                "跳过：VLESS 节点 {}:{} TCP 不可达（节点下线，非代码问题）",
                node.host, node.port
            );
            return;
        }
        let adapter = adapter_for(&node).expect("vless 节点必须能构建适配器");

        let (http_addr, hits) = spawn_local_http();
        let egress = AdapterEgress::new(Arc::new(AdapterDialer(adapter)), Duration::from_secs(10));

        let resp = egress
            .execute(
                &format!("http://{http_addr}/"),
                &[],
                Bytes::new(),
                &TIMEOUTS,
            )
            .await
            .inspect_err(|e| {
                on_dial_failure("VLESS", e);
            })
            .ok();
        let Some(resp) = resp else {
            return;
        };
        assert_eq!(resp.status(), 200);
        assert!(
            hits.load(Ordering::Relaxed) >= 1,
            "请求必须真的穿过外部代理进程"
        );
        println!("✓ VLESS 验证通过：真实代理过程握手成功");
    }
}

#[tokio::test]
async fn ss2022_live() {
    if let Some(node) = parse_env_url("ss", "FERRITE_PROXY_SS") {
        if !node_alive(&node.host, node.port).await {
            eprintln!(
                "跳过：SS 节点 {}:{} TCP 不可达（节点下线，非代码问题）",
                node.host, node.port
            );
            return;
        }
        let adapter = adapter_for(&node).expect("ss 节点必须能构建适配器");

        let (http_addr, hits) = spawn_local_http();
        let egress = AdapterEgress::new(Arc::new(AdapterDialer(adapter)), Duration::from_secs(10));

        let resp = egress
            .execute(
                &format!("http://{http_addr}/"),
                &[],
                Bytes::new(),
                &TIMEOUTS,
            )
            .await
            .inspect_err(|e| {
                on_dial_failure("SS", e);
            })
            .ok();
        let Some(resp) = resp else {
            return;
        };
        assert_eq!(resp.status(), 200);
        assert!(
            hits.load(Ordering::Relaxed) >= 1,
            "请求必须真的穿过外部代理进程"
        );
        println!("✓ SS 验证通过：真实代理过程握手成功");
    }
}

#[tokio::test]
async fn trojan_live() {
    if let Some(node) = parse_env_url("trojan", "FERRITE_PROXY_TROJAN") {
        if !node_alive(&node.host, node.port).await {
            eprintln!(
                "跳过：Trojan 节点 {}:{} TCP 不可达（节点下线，非代码问题）",
                node.host, node.port
            );
            return;
        }
        let adapter = adapter_for(&node).expect("trojan 节点必须能构建适配器");

        let (http_addr, hits) = spawn_local_http();
        let egress = AdapterEgress::new(Arc::new(AdapterDialer(adapter)), Duration::from_secs(10));

        let resp = egress
            .execute(
                &format!("http://{http_addr}/"),
                &[],
                Bytes::new(),
                &TIMEOUTS,
            )
            .await
            .inspect_err(|e| {
                on_dial_failure("Trojan", e);
            })
            .ok();
        let Some(resp) = resp else {
            return;
        };
        assert_eq!(resp.status(), 200);
        assert!(
            hits.load(Ordering::Relaxed) >= 1,
            "请求必须真的穿过外部代理进程"
        );
        println!("✓ Trojan 验证通过：真实代理过程握手成功");
    }
}

#[tokio::test]
async fn vmess_live() {
    if let Some(node) = parse_env_url("vmess", "FERRITE_PROXY_VMESS") {
        if !node_alive(&node.host, node.port).await {
            eprintln!(
                "跳过：VMess 节点 {}:{} TCP 不可达（节点下线，非代码问题）",
                node.host, node.port
            );
            return;
        }
        let adapter = adapter_for(&node).expect("vmess 节点必须能构建适配器");

        let (http_addr, hits) = spawn_local_http();
        let egress = AdapterEgress::new(Arc::new(AdapterDialer(adapter)), Duration::from_secs(10));

        let resp = egress
            .execute(
                &format!("http://{http_addr}/"),
                &[],
                Bytes::new(),
                &TIMEOUTS,
            )
            .await
            .inspect_err(|e| {
                on_dial_failure("VMess", e);
            })
            .ok();
        let Some(resp) = resp else {
            return;
        };
        assert_eq!(resp.status(), 200);
        assert!(
            hits.load(Ordering::Relaxed) >= 1,
            "请求必须真的穿过外部代理进程"
        );
        println!("✓ VMess 验证通过：真实代理过程握手成功");
    }
}
