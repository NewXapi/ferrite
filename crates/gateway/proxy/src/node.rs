//! `node` —— `ProxyNode`：解析后的代理节点

use std::net::SocketAddr;

/// 代理协议
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyScheme {
    Direct,
    Http,
    Socks5,
}

/// HTTP/SOCKS 代理基础认证
#[derive(Debug, Clone)]
pub struct BasicAuth {
    pub user: String,
    pub pass: String,
}

/// 代理节点
#[derive(Debug, Clone)]
pub struct ProxyNode {
    pub id: i64,
    pub scheme: ProxyScheme,
    pub host: String,
    pub port: u16,
    pub auth: Option<BasicAuth>,
    pub channel_ids: Vec<i64>,
    pub priority: i32,
}

impl ProxyNode {
    /// 解析为 `SocketAddr`（host:port）
    pub fn socket_addr(&self) -> Result<SocketAddr, std::net::AddrParseError> {
        format!("{}:{}", self.host, self.port).parse()
    }

    /// 解析原始代理 URL 字符串（接受 `http://user:pass@host:port` / `socks5://...`）
    pub fn parse_url(_url: &str) -> Result<Self, ParseError> {
        // TODO: 解析 url
        unimplemented!("ProxyNode::parse_url")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid proxy url: {0}")]
    Invalid(String),
}
