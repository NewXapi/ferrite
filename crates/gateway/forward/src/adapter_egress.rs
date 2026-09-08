//! `adapter_egress` —— 把任意「拨号函数」桥成 HTTP 出口。
//!
//! [`StreamDialer`] 抽象「给定目标 host/port 返回已完成协议握手的双向流」；
//! [`AdapterEgress`] 在其上叠加 HTTPS 的 TLS 层并用 hyper 发请求，实现
//! [`crate::egress::Egress`]。`gateway-proxy` 侧用 meow 的 `ProxyAdapter`
//! 实现 dialer；测试用本地 TCP 服务实现——两种实现共用同一桥接代码。
//!
//! ## 为什么不用 reqwest
//! reqwest 0.12 的自定义 connector 是 `pub(crate)`（`connect.rs:70`
//! `ConnectorBuilder`），外部无法注入。hyper-util 的 legacy client
//! 支持任意 `tower::Service<Uri>` connector，所以这里直接用 hyper +
//! `crate::egress::Egress`（`ForwardedResponse::from_stream` 公开构造器）。

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use contract::error::NormalizedError;
use futures_util::StreamExt;
use http::Uri;
use http_body_util::{BodyExt, Full};
use hyper::Request;
use hyper::body::Incoming;
use hyper_util::client::legacy::Client;
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::timeout;
use tower::Service;

use crate::egress::{ForwardedResponse, Timeouts, build_header_map, classify_status};

/// 出口拨号器：给定目标 host/port，返回已完成协议握手的双向流。
///
/// 协议握手（SOCKS5 / VLESS / SS / Trojan…）由实现方完成；本模块只管
/// 在流上叠加 TLS 并承载 HTTP。
#[async_trait::async_trait]
pub trait StreamDialer: Send + Sync + std::fmt::Debug {
    /// 拨到 `host:port`。失败返回 `io::Error`（会映射为 502 可重试）。
    async fn dial(&self, host: &str, port: u16) -> std::io::Result<Box<dyn DialedStream>>;
}

/// 已拨号的流。`Unpin + Send + Sync` 让它能被 `TokioIo` 包给 hyper。
pub trait DialedStream: AsyncRead + AsyncWrite + Unpin + Send + Sync {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send + Sync> DialedStream for T {}

/// 拨号流的 hyper 封装：`TokioIo` 提供 Read/Write，外层补 `Connection` 元数据
/// （hyper-util blanket `Connect` 要求响应类型实现它）。
pub struct DialedIo(Box<dyn DialedStream>);

impl hyper::rt::Read for DialedIo {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut TokioIo::new(&mut self.0)).poll_read(cx, buf)
    }
}

impl hyper::rt::Write for DialedIo {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut TokioIo::new(&mut self.0)).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut TokioIo::new(&mut self.0)).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut TokioIo::new(&mut self.0)).poll_shutdown(cx)
    }
}

impl hyper_util::client::legacy::connect::Connection for DialedIo {
    fn connected(&self) -> hyper_util::client::legacy::connect::Connected {
        hyper_util::client::legacy::connect::Connected::new()
    }
}

type DialFuture = Pin<Box<dyn Future<Output = std::io::Result<DialedIo>> + Send>>;

/// hyper connector：把 [`StreamDialer`] 变成 `tower::Service<Uri>`。
///
/// HTTPS 在拨出的明文流上叠加 rustls（标准 WebPKI 校验，SNI = 目标 host）；
/// 连接 + TLS 握手整体受 `connect_timeout` 约束。
#[derive(Clone)]
pub struct DialerConnector {
    dialer: Arc<dyn StreamDialer>,
    connect_timeout: Duration,
}

impl Service<Uri> for DialerConnector {
    type Response = DialedIo;
    type Error = std::io::Error;
    type Future = DialFuture;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // 拨号无共享资源需要预热的，恒就绪
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        let is_tls = uri.scheme_str() == Some("https");
        let host = uri.host().unwrap_or_default().to_string();
        let port = uri.port_u16().unwrap_or(if is_tls { 443 } else { 80 });
        let dialer = Arc::clone(&self.dialer);
        let connect_timeout = self.connect_timeout;

        Box::pin(async move {
            let dial_fut = dialer.dial(&host, port);
            let plain = timeout(connect_timeout, dial_fut).await.map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("dial {host}:{port} timed out"),
                )
            })??;

            let stream: Box<dyn DialedStream> = if is_tls {
                let tls = dial_and_wrap_tls(plain, &host)
                    .await
                    .map_err(|e| io_err(format!("tls to {host}:{port}: {e}")))?;
                Box::new(tls)
            } else {
                plain
            };
            Ok(DialedIo(stream))
        })
    }
}

use std::future::Future;

fn io_err(msg: impl Into<String>) -> std::io::Error {
    std::io::Error::other(msg.into())
}

/// 在明文流上做 TLS 客户端握手（标准 WebPKI 根证书，SNI = host）。
async fn dial_and_wrap_tls(
    plain: Box<dyn DialedStream>,
    host: &str,
) -> std::io::Result<tokio_rustls::client::TlsStream<Box<dyn DialedStream>>> {
    // 依赖树里同时存在 aws-lc-rs（meow / 旧依赖）与 ring 时，rustls 无法
    // 自动挑 provider，需要进程级显式安装一次（幂等：已装则 Err 忽略）。
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
    let server_name = rustls::pki_types::ServerName::try_from(host.to_string())
        .map_err(|e| io_err(format!("invalid server name `{host}`: {e}")))?;
    connector.connect(server_name, plain).await
}

/// 适配器出口：hyper client 持有拨号 connector，[`Egress`] 语义与
/// `ReqwestEgress` 一致（禁重定向、三段超时、first-byte 抢读）。
pub struct AdapterEgress {
    client: Client<DialerConnector, Full<Bytes>>,
    connect_timeout: Duration,
}

impl std::fmt::Debug for AdapterEgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdapterEgress").finish_non_exhaustive()
    }
}

impl AdapterEgress {
    /// 默认 connect 超时（诊断用，与 [`crate::egress::ReqwestEgress::connect_timeout`] 对齐）。
    pub fn connect_timeout(&self) -> Duration {
        self.connect_timeout
    }

    /// 用拨号器构造出口 client。连接池由 hyper-util 自带（per-host 复用）。
    pub fn new(dialer: Arc<dyn StreamDialer>, connect_timeout: Duration) -> Self {
        let connector = DialerConnector {
            dialer,
            connect_timeout,
        };
        let client = Client::builder(TokioExecutor::new()).build(connector);
        Self {
            client,
            connect_timeout,
        }
    }
}

/// hyper 响应体 → `ForwardedResponse` 的字节流（io::Error 边界对齐 reqwest 版）。
fn incoming_to_stream(
    body: Incoming,
) -> Pin<Box<dyn futures_util::Stream<Item = Result<Bytes, std::io::Error>> + Send>> {
    Box::pin(BodyExt::into_stream(body).map(|chunk| match chunk {
        Ok(frame) => Ok(frame.into_data().unwrap_or_default()),
        Err(e) => Err(std::io::Error::other(format!("upstream stream error: {e}"))),
    }))
}

impl crate::egress::Egress for AdapterEgress {
    fn execute<'a>(
        &'a self,
        url: &'a str,
        headers: &'a [(String, String)],
        body: Bytes,
        timeouts: &'a Timeouts,
    ) -> Pin<Box<dyn Future<Output = Result<ForwardedResponse, NormalizedError>> + Send + 'a>> {
        Box::pin(self.execute_impl(url, headers, body, timeouts))
    }
}

impl AdapterEgress {
    /// 实际的转发逻辑；`Egress::execute` 只是包一层 Box::pin。
    async fn execute_impl(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: Bytes,
        timeouts: &Timeouts,
    ) -> Result<ForwardedResponse, NormalizedError> {
        let total_timeout = Duration::from_millis(timeouts.total_ms);
        let first_byte_timeout = Duration::from_millis(timeouts.first_byte_ms);

        let uri: Uri = url.parse().map_err(|e| NormalizedError {
            code: contract::error::code::UPSTREAM_ERROR,
            status: 502,
            retryable: false,
            message: format!("invalid upstream url `{url}`: {e}"),
        })?;
        let header_map = build_header_map(headers)?;

        let mut builder = Request::builder()
            .method(hyper::Method::POST)
            .uri(uri)
            .header(hyper::header::CONTENT_LENGTH, body.len());
        {
            let hm = builder.headers_mut().expect("request not yet built");
            for (k, v) in header_map.iter() {
                hm.append(k, v.clone());
            }
        }
        let req = builder.body(Full::new(body)).map_err(|e| NormalizedError {
            code: contract::error::code::UPSTREAM_ERROR,
            status: 502,
            retryable: false,
            message: format!("build request: {e}"),
        })?;

        let send_fut = self.client.request(req);
        let resp = timeout(total_timeout, send_fut)
            .await
            .map_err(|_| NormalizedError {
                code: contract::error::code::UPSTREAM_ERROR,
                status: 504,
                retryable: true,
                message: "upstream total timeout".into(),
            })?
            .map_err(|e| {
                // hyper-util legacy Error 的 connect/timeout 判定：is_connect 有 API，
                // timeout 在错误链的 io ErrorKind 里（dial 阶段我们用 TimedOut 构造）
                let is_timeout = std::error::Error::source(&e)
                    .and_then(|s| s.downcast_ref::<std::io::Error>())
                    .is_some_and(|io| io.kind() == std::io::ErrorKind::TimedOut)
                    || e.to_string().to_lowercase().contains("timed out");
                NormalizedError {
                    code: contract::error::code::UPSTREAM_ERROR,
                    status: if is_timeout { 504 } else { 502 },
                    retryable: e.is_connect() || is_timeout,
                    message: e.to_string(),
                }
            })?;

        let status = resp.status().as_u16();
        let content_type = resp
            .headers()
            .get(hyper::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        // 非 2xx → 读完整 body 归类错误（与 ReqwestEgress 相同语义）
        if !resp.status().is_success() {
            let preview = match BodyExt::collect(resp.into_body()).await {
                Ok(c) => {
                    let b = c.to_bytes();
                    String::from_utf8_lossy(&b[..b.len().min(2048)]).into_owned()
                }
                Err(_) => String::new(),
            };
            return Err(classify_status(status, preview));
        }

        // 2xx → first-byte 抢读 + prepend（语义对齐 ReqwestEgress）
        let mut body_stream = incoming_to_stream(resp.into_body());
        let first_chunk = timeout(first_byte_timeout, body_stream.next())
            .await
            .map_err(|_| NormalizedError {
                code: contract::error::code::UPSTREAM_ERROR,
                status: 504,
                retryable: true,
                message: "upstream first-byte timeout".into(),
            })?;

        let body: Pin<Box<dyn futures_util::Stream<Item = Result<Bytes, std::io::Error>> + Send>> =
            match first_chunk {
                None => Box::pin(futures_util::stream::empty()),
                Some(Ok(first)) => {
                    let head = futures_util::stream::once(async move { Ok(first) });
                    Box::pin(head.chain(body_stream))
                }
                Some(Err(e)) => {
                    return Err(NormalizedError {
                        code: contract::error::code::UPSTREAM_ERROR,
                        status: 502,
                        retryable: true,
                        message: format!("upstream stream error: {e}"),
                    });
                }
            };
        Ok(ForwardedResponse::from_stream(status, content_type, body))
    }
}
