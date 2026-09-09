//! `adapter_egress` 桥接测试 —— 用本地 HTTP 服务验证「拨号器 → HTTP 出口」全链。
//!
//! 不需要真实网络与代理节点：`StreamDialer` 的实现方直接 `TcpStream::connect`
//! 本地回显服务，`AdapterEgress` 在其上发真实 HTTP/1.1 请求。只要这条链
//! 成立，把 dialer 换成 meow 的 `ProxyAdapter::dial_tcp` 就是生产路径。
//!
//! TLS 路径（HTTPS）需要真证书或自签 CA，本地单测不覆盖——由 examples smoke
//! 用真实上游验证。

use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use bytes::Bytes;

use forward::adapter_egress::{AdapterEgress, DialedStream, StreamDialer};
use forward::egress::Egress;
use forward::egress::Timeouts;
use futures_util::StreamExt;

/// 同步线程里的极简 HTTP/1.1 服务：读到请求头结束即回 200。
/// 返回监听地址与命中计数（供断言「请求确实经过 dialer」）。
fn spawn_local_http() -> (std::net::SocketAddr, Arc<AtomicUsize>) {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let addr = listener.local_addr().expect("local_addr");
    let hits = Arc::new(AtomicUsize::new(0));
    let hits2 = Arc::clone(&hits);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            hits2.fetch_add(1, Ordering::Relaxed);
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                let mut got = Vec::new();
                // 读到 \r\n\r\n（请求头结束）为止；本地环回，很快
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
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(resp.as_bytes());
                let _ = stream.write_all(body);
            });
        }
    });
    (addr, hits)
}

/// 直连本地服务的 dialer —— 替身：生产里这个角色由 meow ProxyAdapter 扮演。
#[derive(Debug)]
struct LocalDialer(std::net::SocketAddr);

#[async_trait::async_trait]
impl StreamDialer for LocalDialer {
    async fn dial(&self, host: &str, port: u16) -> std::io::Result<Box<dyn DialedStream>> {
        // 忽略调用方的 host/port：测试里固定连本地服务。
        // 这样还能断言「URI 的 host 与 dial 目标无关」——桥不做 DNS，
        // 目标解析完全交给 dialer（生产里是代理协议服务器）。
        let _ = (host, port);
        let conn = tokio::net::TcpStream::connect(self.0).await?;
        Ok(Box::new(conn))
    }
}

/// 一定失败的 dialer —— 模拟代理节点不可达。
#[derive(Debug)]
struct RefusingDialer;

#[async_trait::async_trait]
impl StreamDialer for RefusingDialer {
    async fn dial(&self, _host: &str, _port: u16) -> std::io::Result<Box<dyn DialedStream>> {
        Err(std::io::Error::other("dial refused (simulated)"))
    }
}

/// 故意慢的 dialer —— 验证 connect 超时生效。
#[derive(Debug)]
struct SlowDialer(Duration);

#[async_trait::async_trait]
impl StreamDialer for SlowDialer {
    async fn dial(&self, _host: &str, _port: u16) -> std::io::Result<Box<dyn DialedStream>> {
        tokio::time::sleep(self.0).await;
        Err(std::io::Error::other("slow dial should have timed out"))
    }
}

fn headers_for(body_len: usize) -> Vec<(String, String)> {
    vec![
        ("content-type".into(), "application/json".into()),
        ("content-length".into(), body_len.to_string()),
    ]
}

/// 正常路径：dialer 连本地服务 → Egress 发真实 HTTP → 200 + body 正确，
/// 且请求头真的被带上了（服务端回显没有 header 计数，改为断言 hits 计数）。
#[tokio::test]
async fn egress_sends_http_over_dialer() {
    let (addr, hits) = spawn_local_http();
    let egress = AdapterEgress::new(Arc::new(LocalDialer(addr)), Duration::from_secs(5));

    let body = Bytes::from_static(br#"{"model":"probe"}"#);
    let resp = egress
        .execute(
            &format!("http://{}/v1/chat/completions", addr),
            &headers_for(body.len()),
            body,
            &Timeouts::default(),
        )
        .await
        .expect("egress over local dialer should succeed");

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.content_type(), "text/plain");
    let mut stream = resp.into_body_stream();
    let mut collected = Vec::new();
    while let Some(chunk) = stream.next().await {
        collected.extend_from_slice(&chunk.expect("body chunk"));
    }
    assert_eq!(collected, b"local-ok");
    assert_eq!(hits.load(Ordering::Relaxed), 1, "请求必须真的经过了 dialer");
}

/// dial 失败 → 502 且 retryable=true（对齐 classify_status 的传输错误语义）。
/// retry 层靠这个语义换候选节点，语义错了会导致坏节点不冷却。
#[tokio::test]
async fn dial_failure_is_retryable_502() {
    let egress = AdapterEgress::new(Arc::new(RefusingDialer), Duration::from_secs(5));
    let err = egress
        .execute(
            "http://upstream.invalid/v1/chat/completions",
            &headers_for(2),
            Bytes::from_static(b"{}"),
            &Timeouts::default(),
        )
        .await
        .expect_err("dial 失败必须报错");
    assert_eq!(err.status, 502, "got: {err:?}");
    assert!(err.retryable, "传输失败必须可重试: {err:?}");
}

/// dial 慢于 connect_timeout → 超时、504、可重试。
#[tokio::test]
async fn slow_dial_hits_connect_timeout() {
    let egress = AdapterEgress::new(
        Arc::new(SlowDialer(Duration::from_secs(30))),
        Duration::from_millis(200), // connect_timeout 覆盖 dial
    );
    let err = egress
        .execute(
            "http://upstream.invalid/v1/chat/completions",
            &headers_for(2),
            Bytes::from_static(b"{}"),
            &Timeouts::default(),
        )
        .await
        .expect_err("慢 dial 必须超时");
    assert_eq!(err.status, 504, "超时映射 504: {err:?}");
    assert!(err.retryable);
}

/// 流式：服务端响应在首 chunk 之后还有数据 → into_body_stream 拿全。
/// first-byte 抢读 + prepend 逻辑不能丢字节（ReqwestEgress 的关键语义）。
#[tokio::test]
async fn body_stream_not_truncated() {
    // 复用 spawn_local_http：固定 body "local-ok"（8 字节）足够验证
    let (addr, _hits) = spawn_local_http();
    let egress = AdapterEgress::new(Arc::new(LocalDialer(addr)), Duration::from_secs(5));
    let resp = egress
        .execute(
            &format!("http://{}/", addr),
            &headers_for(0),
            Bytes::new(),
            &Timeouts::default(),
        )
        .await
        .expect("ok");
    let mut stream = resp.into_body_stream();
    let mut all = Vec::new();
    while let Some(chunk) = stream.next().await {
        all.extend_from_slice(&chunk.expect("chunk"));
    }
    assert_eq!(all, b"local-ok", "body 不能丢首字节或尾字节");
}
