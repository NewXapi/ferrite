//! smoke：meow 适配器 → adapter_egress 桥 → 真实上游。
//!
//! 验证 PR #84 的完整承诺：`ProxyAdapter::dial_tcp` 产出的流经
//! `AdapterEgress` 能承载真实 HTTPS 请求（dial + rustls TLS + hyper）。
//!
//! 运行：`cargo run --example adapter_dial_smoke -p gateway-proxy`
//! 前置：本机 sing-box mixed 入站 `127.0.0.1:7890`（或改下面的端口）。

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use forward::adapter_egress::AdapterEgress;
use forward::egress::{Egress, Timeouts};
use gateway_proxy::adapter::adapter_for;
use gateway_proxy::node::{BasicAuth, ProxyNode, ProxyScheme};

#[tokio::main]
async fn main() {
    // 隔离系统代理：确保流量真的走 adapter 拨号，而不是环境变量兜底。
    for var in [
        "ALL_PROXY",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "all_proxy",
        "http_proxy",
        "https_proxy",
    ] {
        // 单线程 example 进程独占环境变量，无并发读取者，安全。
        unsafe { std::env::remove_var(var) };
    }

    let node = ProxyNode {
        id: 1,
        scheme: ProxyScheme::Socks5,
        host: "127.0.0.1".into(),
        port: 7890,
        auth: Some(BasicAuth {
            user: String::new(),
            pass: String::new(),
        }),
        channel_keys: vec!["smoke".into()],
        opts: None,
        priority: 10,
    };
    let Some(adapter) = adapter_for(&node) else {
        panic!("socks5 节点必须能映射成 adapter");
    };
    println!(
        "adapter: type={:?} addr={}",
        adapter.adapter_type(),
        adapter.addr()
    );

    let egress = AdapterEgress::new(
        Arc::new(bridge::AdapterDialer(adapter)),
        Duration::from_secs(10),
    );

    println!("=== 1) HTTPS 经 adapter dial + rustls ===");
    let resp = egress
        .execute(
            "https://httpbin.org/post",
            &[("accept".into(), "application/json".into())],
            Bytes::new(),
            &Timeouts::default(),
        )
        .await
        .expect("HTTPS over adapter dial must work");
    let (status, content_type) = (resp.status(), resp.content_type().to_string());
    println!("status={status} content_type={content_type}");
    let mut stream = resp.into_body_stream();
    let mut body = Vec::new();
    while let Some(chunk) = futures_util::StreamExt::next(&mut stream).await {
        body.extend_from_slice(&chunk.expect("body chunk"));
    }
    let text = String::from_utf8_lossy(&body);
    println!("body: {}", text.trim());
    assert!(
        text.contains("origin"),
        "httpbin /post 必须回显出口 IP: {text}"
    );
    assert_eq!(status, 200, "httpbin 必须回 200");

    println!("\n=== SMOKE PASSED: adapter dial → TLS → HTTP 全链真实可用 ===");
}

/// 复刻 forward::stage::AdapterDialer——example 不能引用 stage 内部私有类型。
mod bridge {
    use std::sync::Arc;

    use forward::adapter_egress::{DialedStream, StreamDialer};
    use gateway_proxy::{ProxyAdapter, ProxyConn, tcp_metadata};

    pub struct AdapterDialer(pub Arc<dyn ProxyAdapter>);

    impl std::fmt::Debug for AdapterDialer {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("AdapterDialer").finish_non_exhaustive()
        }
    }

    #[async_trait::async_trait]
    impl StreamDialer for AdapterDialer {
        async fn dial(&self, host: &str, port: u16) -> std::io::Result<Box<dyn DialedStream>> {
            let meta = tcp_metadata(host, port);
            let conn = self
                .0
                .dial_tcp(&meta)
                .await
                .map_err(std::io::Error::other)?;
            Ok(Box::new(MeowConn(conn)))
        }
    }

    pub struct MeowConn(Box<dyn ProxyConn>);

    impl tokio::io::AsyncRead for MeowConn {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::pin::Pin::new(&mut self.0).poll_read(cx, buf)
        }
    }

    impl tokio::io::AsyncWrite for MeowConn {
        fn poll_write(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<std::io::Result<usize>> {
            std::pin::Pin::new(&mut self.0).poll_write(cx, buf)
        }
        fn poll_flush(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::pin::Pin::new(&mut self.0).poll_flush(cx)
        }
        fn poll_shutdown(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::pin::Pin::new(&mut self.0).poll_shutdown(cx)
        }
    }
}
