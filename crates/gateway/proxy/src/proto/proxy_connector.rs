use async_trait::async_trait;
use std::fmt;

use crate::proto::{AsyncStream, NetLocation, ResolvedLocation};

/// 代理连接器 trait
///
/// 本 trait 移植自 shoes MIT 许可的 tcp/proxy_connector.rs，适配本 crate。
/// 服务于 reqwest 无法覆盖的协议（vless/vmess/ss/trojan 等）。
/// 客户端链基础设施仅处理 TCP 流，不包含 UDP 方法。
///
/// ## 职责
/// - 返回代理服务器位置
/// - 在现有流上设置协议握手，返回包装后的流
///
/// ## 与 reqwest 的关系
/// reqwest 内置支持 HTTP CONNECT 和 SOCKS5，本 trait 用于更复杂的协议。
#[async_trait]
pub trait ProxyConnector: Send + Sync + fmt::Debug {
    /// 返回代理服务器地址
    fn proxy_location(&self) -> &NetLocation;

    /// 在现有流上设置协议
    ///
    /// # 参数
    /// - `stream`: 现有传输流
    /// - `target`: 流量应该到达的目的地
    ///
    /// # 返回
    /// 包装了协议的异步流
    async fn setup_tcp_stream(
        &self,
        stream: Box<dyn AsyncStream>,
        target: &ResolvedLocation,
    ) -> std::io::Result<Box<dyn AsyncStream>>;
}

/// 带有指定位置的 ProxyConnector
#[derive(Debug)]
pub struct WithProxyLocation<T: ProxyConnector> {
    location: NetLocation,
    connector: T,
}

impl<T: ProxyConnector> WithProxyLocation<T> {
    pub fn new(location: NetLocation, connector: T) -> Self {
        Self {
            location,
            connector,
        }
    }
}

#[async_trait]
impl<T: ProxyConnector + fmt::Debug + Send + Sync> ProxyConnector for WithProxyLocation<T> {
    fn proxy_location(&self) -> &NetLocation {
        &self.location
    }

    async fn setup_tcp_stream(
        &self,
        stream: Box<dyn AsyncStream>,
        target: &ResolvedLocation,
    ) -> std::io::Result<Box<dyn AsyncStream>> {
        self.connector.setup_tcp_stream(stream, target).await
    }
}
