//! 真实代理节点的 opt-in 验证 —— 用真进程（sing-box / mihomo / 任意 SOCKS5 服务）
//! 替掉 `adapter_sim.rs` 里的 mock server，跑同一条链：
//! `adapter_for` → meow 真握手 → `AdapterEgress` 桥 → HTTP。
//!
//! # 怎么跑
//!
//! ```sh
//! # 本机 sing-box mixed 入站在 7890
//! FERRITE_PROXY_SOCKS5=127.0.0.1:7890 cargo test -p forward --test real_node
//! ```
//!
//! 环境变量缺省时**直接通过并打印跳过原因**：CI 没有代理进程，不能因此变红。
//! 上游目标是测试内起的本地 HTTP 服务，所以即便真拨也**不出公网**——
//! 验证的是「真实代理进程的协议握手 + 我们的桥」，不是外网连通性。

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

/// 极简 HTTP/1.1 服务：读到头结束即回 200 + `real-ok`，带命中计数。
fn spawn_local_http() -> (SocketAddr, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let addr = listener.local_addr().expect("local_addr");
    let hits = Arc::new(AtomicUsize::new(0));
    let hits2 = Arc::clone(&hits);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            hits2.fetch_add(1, Ordering::Relaxed);
            std::thread::spawn(move || {
                let mut got = Vec::new();
                let mut buf = [0u8; 4096];
                loop {
                    match stream.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            got.extend_from_slice(&buf[..n]);
                            if got.windows(4).any(|w| w == b"\r\n\r\n") {
                                break;
                            }
                        }
                    }
                }
                let body = b"real-ok";
                let _ = stream.write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .as_bytes(),
                );
                let _ = stream.write_all(body);
            });
        }
    });
    (addr, hits)
}

/// 真实 SOCKS5 节点（sing-box mixed 入站等）跑完整出口链。
///
/// 断言本地 HTTP 被命中 —— 证明字节真的穿过了外部代理进程再回到我们手里，
/// 而不是 bridge 偷偷直连。
#[tokio::test]
async fn egress_over_real_socks5_node() {
    let Ok(proxy) = std::env::var("FERRITE_PROXY_SOCKS5") else {
        eprintln!(
            "跳过：设 FERRITE_PROXY_SOCKS5=<host:port>（如本机 sing-box 127.0.0.1:7890）才会真拨"
        );
        return;
    };
    let proxy: SocketAddr = proxy
        .parse()
        .expect("FERRITE_PROXY_SOCKS5 必须是 host:port");

    let (http_addr, hits) = spawn_local_http();
    let node = ProxyNode {
        id: 1,
        scheme: ProxyScheme::Socks5,
        host: proxy.ip().to_string(),
        port: proxy.port(),
        auth: None,
        vless: None,
        channel_keys: vec!["openai".to_string()],
        priority: 0,
    };
    let adapter = adapter_for(&node).expect("socks5 节点必须映射出适配器");
    let egress = AdapterEgress::new(Arc::new(AdapterDialer(adapter)), Duration::from_secs(10));

    let resp = egress
        .execute(
            &format!("http://{http_addr}/real"),
            &[],
            Bytes::new(),
            &TIMEOUTS,
        )
        .await
        .expect("经真实 SOCKS5 节点的 HTTP 出口必须成功");
    assert_eq!(resp.status(), 200);
    assert!(
        hits.load(Ordering::Relaxed) >= 1,
        "请求必须真的穿过外部代理进程"
    );
}
