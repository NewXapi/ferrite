//! `dialer` —— `Dialer` trait + 多种实现
//!
//! 每种代理方案对应一个 dialer 实现。`forward` 通过 `ProxyPool::pick` 选出
//! 代理节点，再用对应 dialer 拨号。

use std::net::SocketAddr;
use tokio::net::TcpStream;
use async_trait::async_trait;
use super::node::ProxyNode;

#[async_trait]
pub trait Dialer: Send + Sync {
    /// 拨号到目标地址（经代理或直连）
    async fn dial(&self, target: SocketAddr) -> Result<TcpStream, std::io::Error>;
}

/// 直连 dialer
pub struct DirectDialer;

#[async_trait]
impl Dialer for DirectDialer {
    async fn dial(&self, target: SocketAddr) -> Result<TcpStream, std::io::Error> {
        TcpStream::connect(target).await
    }
}

/// SOCKS5 dialer
pub struct Socks5Dialer {
    pub proxy: ProxyNode,
}

#[async_trait]
impl Dialer for Socks5Dialer {
    async fn dial(&self, target: SocketAddr) -> Result<TcpStream, std::io::Error> {
        // TODO: SOCKS5 握手 + CONNECT
        unimplemented!("Socks5Dialer::dial")
    }
}

/// HTTP CONNECT dialer
pub struct HttpConnectDialer {
    pub proxy: ProxyNode,
}

#[async_trait]
impl Dialer for HttpConnectDialer {
    async fn dial(&self, target: SocketAddr) -> Result<TcpStream, std::io::Error> {
        // TODO: HTTP CONNECT 隧道
        unimplemented!("HttpConnectDialer::dial")
    }
}
