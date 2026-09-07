//! `tls` —— 最小 rustls 客户端封装（Trojan 需要 TLS 内传输）。
//!
//! 只做标准 WebPKI 校验 + 系统根证书，无 CA 定制 / client cert / 指纹伪装
//! （那些是 Reality/伪装场景，PR4 按需）。用 `tokio-rustls` 完成异步握手，
//! 返回 [`AsyncStream`]。

use std::sync::Arc;

use tokio::net::TcpStream;
use tokio_rustls::TlsConnector as TokioTlsConnector;
use tokio_rustls::rustls;

use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::ReadBuf;

use crate::proto::async_stream::{AsyncPing, AsyncStream};
/// tokio-rustls 的 TlsStream 实现 AsyncRead/AsyncWrite 但没有 AsyncPing；
/// 包一层补齐 AsyncStream 的 trait bound。
pub struct TlsAsyncStream(pub tokio_rustls::client::TlsStream<TcpStream>);

impl AsyncPing for TlsAsyncStream {
    fn supports_ping(&self) -> bool {
        false
    }
    fn poll_write_ping(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<bool>> {
        Poll::Ready(Ok(false))
    }
}

impl tokio::io::AsyncRead for TlsAsyncStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl tokio::io::AsyncWrite for TlsAsyncStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

impl AsyncStream for TlsAsyncStream {}

/// TLS 客户端连接器。
#[derive(Debug, Clone)]
pub struct TlsClientHandler {
    /// 覆写的 SNI（缺省用目标 host）
    server_name: Option<String>,
}

impl TlsClientHandler {
    pub fn new(server_name: Option<String>) -> Self {
        Self { server_name }
    }

    /// 建立 TLS 连接：TCP 流 → ClientHello → 返回可读写的 TLS 流。
    ///
    /// # 参数
    /// - `stream`：已连到 Trojan 服务器的 TCP 流
    /// - `host`：SNI 用的目标主机名（证书校验对象）
    pub async fn wrap(
        &self,
        stream: TcpStream,
        host: &str,
    ) -> std::io::Result<Box<dyn AsyncStream>> {
        let config = Arc::new(client_config()?);
        let sni = self.server_name.as_deref().unwrap_or(host);
        let name = rustls::pki_types::ServerName::try_from(sni.to_string()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid SNI: {e}"),
            )
        })?;

        let connector = TokioTlsConnector::from(config);
        let tls = connector.connect(name, stream).await?;
        Ok(Box::new(TlsAsyncStream(tls)))
    }
}

/// 标准 WebPKI 客户端配置：系统根证书 + 默认 ALPN（空）。
fn client_config() -> std::io::Result<rustls::ClientConfig> {
    let root_store = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    Ok(rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth())
}
