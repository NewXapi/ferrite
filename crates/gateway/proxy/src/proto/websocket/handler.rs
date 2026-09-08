//! WebSocket 客户端连接器（ported from shoes, MIT）。
//!
//! 将 [`AsyncStream`] 包成 WS 隧道（帧编解码见 [`super::stream::WebsocketStream`]），
//! 供 VMess/VLESS over WS 组合使用。

use async_trait::async_trait;

use super::stream::WebsocketStream;
use crate::proto::address::{NetLocation, ResolvedLocation};
use crate::proto::async_stream::AsyncStream;
use crate::proto::proxy_connector::ProxyConnector;

/// WS 隧道目标参数。
#[derive(Debug, Clone)]
pub struct WebsocketServerTarget {
    pub host: String,
    pub path: String,
}

/// WS 客户端连接器：把底层流包成 WS 隧道后转发给内层 connector（如 VMess）。
#[derive(Debug)]
pub struct WebsocketTcpClientHandler {
    target: WebsocketServerTarget,
}

impl WebsocketTcpClientHandler {
    pub fn new(host: String, path: String) -> Self {
        Self {
            target: WebsocketServerTarget { host, path },
        }
    }
}

#[async_trait]
impl ProxyConnector for WebsocketTcpClientHandler {
    fn proxy_location(&self) -> &NetLocation {
        unreachable!("WS 是传输层，proxy_location 由内层 connector 提供")
    }

    async fn setup_tcp_stream(
        &self,
        stream: Box<dyn AsyncStream>,
        target: &ResolvedLocation,
    ) -> std::io::Result<Box<dyn AsyncStream>> {
        let _ = (&self.target, target); // WS 握手参数由 manager 组合层处理
        // WS 握手头（host/path）由上层 HTTP Upgrade 处理；此处只做帧封装
        Ok(Box::new(WebsocketStream::new(
            stream,
            true, // is_client
            super::WebsocketPingType::None,
            &[],
        )))
    }
}
