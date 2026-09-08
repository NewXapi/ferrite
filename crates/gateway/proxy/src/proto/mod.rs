//! 本模块移植自 MIT 许可的 shoes 项目客户端链基础设施。
//! 提供零进程多协议出口接口，与 reqwest 的边界定义。
//!
//! # 模块职责
//! - 定义异步流协议接口（AsyncStream、AsyncPing）
//! - 提供地址解析与网络位置表示（Address、NetLocation、ResolvedLocation）
//! - 实现流读取器（StreamReader）
//! - 定义代理连接器 trait（ProxyConnector）
//! - 各协议客户端握手：Shadowsocks / Trojan / VMess / VLESS（+ WebSocket 传输）
//!
//! 具体协议握手由各子模块实现；`ProxyManager` 按节点 scheme 分派。
pub mod address;
pub mod async_stream;
pub mod proxy_connector;
pub mod stream_reader;

// 导出核心类型供外部模块使用
pub use address::{Address, NetLocation, ResolvedLocation};
pub use async_stream::{AsyncPing, AsyncStream};
pub use proxy_connector::ProxyConnector;
pub use stream_reader::StreamReader;

// ---- PR2: Shadowsocks / Trojan 客户端握手（shoes MIT 移植）----
pub mod shadowsocks;
pub mod socks_addr;
pub mod tls;
pub mod trojan;

pub use shadowsocks::ShadowsocksProxyConnector;
pub use trojan::TrojanProxyConnector;

/// ---- PR3: VMess / VLESS + WebSocket 客户端握手（shoes MIT 移植）----
pub mod vless;
pub mod vmess;
pub mod websocket;

pub use vless::VlessProxyConnector;
pub use vmess::{DataCipher, VmessProxyConnector, new_vmess_connector};
pub use websocket::WebsocketTcpClientHandler;
