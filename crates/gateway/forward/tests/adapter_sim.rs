//! 真 meow adapter 的 in-process 仿真 —— mock SOCKS5 / HTTP-CONNECT 服务端 +
//! 真 `Socks5Adapter` / `HttpAdapter`（`gateway_proxy::adapter_for`）+
//! `AdapterEgress` 桥，全链零外网，CI 可跑。
//!
//! 边界：密码学协议（SS/VMess/VLESS/Trojan/Reality）的握手需要真服务端实现，
//! meow 生态没有公开 server crate，留给真实节点 smoke（examples/adapter_dial_smoke）。

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use bytes::Bytes;

use forward::adapter_egress::{AdapterEgress, DialedStream, StreamDialer};
use forward::egress::{Egress, Timeouts};
use gateway_proxy::{ProxyAdapter, ProxyNode, ProxyScheme, adapter_for, tcp_metadata};

/// 测试专用超时：total 10s，卡住时快速失败（默认 300s 太慢）。
const TEST_TIMEOUTS: Timeouts = Timeouts {
    connect_ms: 5_000,
    first_byte_ms: 10_000,
    total_ms: 10_000,
};

/// 把真 [`ProxyAdapter`] 包成 bridge 的 [`StreamDialer`]。
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
            .map_err(|e| std::io::Error::other(format!("dial {host}:{port} via adapter: {e}")))?;
        Ok(Box::new(conn))
    }
}

fn node(scheme: ProxyScheme, port: u16) -> ProxyNode {
    ProxyNode {
        id: 1,
        scheme,
        host: "127.0.0.1".to_string(),
        port,
        auth: None,
        channel_keys: vec!["sim".to_string()],
        priority: 0,
    }
}

/// 极简 HTTP/1.1 服务：读到头结束即回 200 + `local-ok`，带命中计数。
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
                let body = b"local-ok";
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

/// 极简 SOCKS5 服务端（no-auth）：greeting → CONNECT → 双向 relay。
fn spawn_socks5_proxy() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let addr = listener.local_addr().expect("local_addr");
    std::thread::spawn(move || {
        for client in listener.incoming() {
            let Ok(client) = client else { continue };
            std::thread::spawn(move || {
                let _ = socks5_handle(client);
            });
        }
    });
    addr
}

fn socks5_handle(mut client: TcpStream) -> std::io::Result<()> {
    // greeting: 05 nmethods methods
    let mut head = [0u8; 2];
    client.read_exact(&mut head)?;
    let mut methods = vec![0u8; head[1] as usize];
    client.read_exact(&mut methods)?;
    client.write_all(&[0x05, 0x00])?; // 选 no-auth

    // CONNECT: 05 01 00 atyp ...
    let mut req = [0u8; 4];
    client.read_exact(&mut req)?;
    let target: SocketAddr = match req[3] {
        0x01 => {
            let mut ip_port = [0u8; 6];
            client.read_exact(&mut ip_port)?;
            SocketAddr::V4(std::net::SocketAddrV4::new(
                std::net::Ipv4Addr::new(ip_port[0], ip_port[1], ip_port[2], ip_port[3]),
                u16::from_be_bytes([ip_port[4], ip_port[5]]),
            ))
        }
        0x03 => {
            let len = {
                let mut b = [0u8; 1];
                client.read_exact(&mut b)?;
                b[0] as usize
            };
            let mut name = vec![0u8; len];
            client.read_exact(&mut name)?;
            let mut port = [0u8; 2];
            client.read_exact(&mut port)?;
            format!(
                "{}:{}",
                String::from_utf8_lossy(&name),
                u16::from_be_bytes(port)
            )
            .parse()
            .map_err(std::io::Error::other)?
        }
        other => return Err(std::io::Error::other(format!("unsupported atyp {other}"))),
    };

    let target_conn = TcpStream::connect(target)?;
    client.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])?;
    relay(&mut client, target_conn)
}

/// 极简 HTTP CONNECT 代理服务端：读 CONNECT 行 → 200 → 双向 relay。
fn spawn_http_connect_proxy() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let addr = listener.local_addr().expect("local_addr");
    std::thread::spawn(move || {
        for client in listener.incoming() {
            let Ok(client) = client else { continue };
            std::thread::spawn(move || {
                let _ = http_connect_handle(client);
            });
        }
    });
    addr
}

fn http_connect_handle(mut client: TcpStream) -> std::io::Result<()> {
    let mut got = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = client.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        got.extend_from_slice(&buf[..n]);
        if got.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    let line_end = got.iter().position(|b| *b == b'\r').unwrap_or(got.len());
    let first_line = String::from_utf8_lossy(&got[..line_end]);
    let target = first_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| std::io::Error::other("no CONNECT target"))?;
    let target_addr = target
        .parse::<SocketAddr>()
        .map_err(std::io::Error::other)?;
    let target_conn = TcpStream::connect(target_addr)?;
    client.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")?;
    relay(&mut client, target_conn)
}

/// 双向 relay 直到任一侧 EOF。
fn relay(client: &mut TcpStream, mut target: TcpStream) -> std::io::Result<()> {
    let mut client_up = client.try_clone()?;
    let mut target_up = target.try_clone()?;
    let up = std::thread::spawn(move || std::io::copy(&mut client_up, &mut target_up));
    let _ = std::io::copy(&mut target, client);
    let _ = up.join();
    Ok(())
}

/// 全链：`Socks5Adapter`（真握手）→ mock SOCKS5 → `AdapterEgress` → 本地 HTTP。
/// 断言命中本地服务，证明流量确实穿过真 adapter 与 bridge。
#[tokio::test]
async fn egress_over_real_socks5_adapter() {
    let (http_addr, hits) = spawn_local_http();
    let proxy_addr = spawn_socks5_proxy();

    let adapter = adapter_for(&node(ProxyScheme::Socks5, proxy_addr.port()))
        .expect("socks5 node must map to an adapter");
    let egress = AdapterEgress::new(Arc::new(AdapterDialer(adapter)), Duration::from_secs(5));

    let resp = egress
        .execute(
            &format!("http://{http_addr}/sim"),
            &[],
            Bytes::new(),
            &TEST_TIMEOUTS,
        )
        .await
        .expect("HTTP via real SOCKS5 adapter should succeed");
    assert_eq!(resp.status(), 200);
    assert!(
        hits.load(Ordering::Relaxed) >= 1,
        "请求必须穿过 SOCKS5 mock"
    );
}

/// 全链：`HttpAdapter`（真 CONNECT 握手）→ mock CONNECT → `AdapterEgress` → 本地 HTTP。
#[tokio::test]
async fn egress_over_real_http_connect_adapter() {
    let (http_addr, hits) = spawn_local_http();
    let proxy_addr = spawn_http_connect_proxy();

    let adapter = adapter_for(&node(ProxyScheme::Http, proxy_addr.port()))
        .expect("http node must map to an adapter");
    let egress = AdapterEgress::new(Arc::new(AdapterDialer(adapter)), Duration::from_secs(5));

    let resp = egress
        .execute(
            &format!("http://{http_addr}/sim"),
            &[],
            Bytes::new(),
            &TEST_TIMEOUTS,
        )
        .await
        .expect("HTTP via real CONNECT adapter should succeed");
    assert_eq!(resp.status(), 200);
    assert!(
        hits.load(Ordering::Relaxed) >= 1,
        "请求必须穿过 CONNECT mock"
    );
}
