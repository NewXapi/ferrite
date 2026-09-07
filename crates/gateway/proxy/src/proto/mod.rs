//! 本模块移植自 MIT 许可的 shoes 项目客户端链基础设施。
//! 提供零进程多协议出口接口，与 reqwest 的边界定义。
//!
//! # 模块职责
//! - 定义异步流协议接口（AsyncStream、AsyncPing）
//! - 提供地址解析与网络位置表示（Address、NetLocation、ResolvedLocation）
//! - 实现流读取器（StreamReader）
//! - 定义代理连接器 trait（ProxyConnector）
//!
//! 本 crate 不实现具体协议握手，仅提供接口供上层使用。
pub mod address;
pub mod async_stream;
pub mod proxy_connector;
pub mod stream_reader;

// 导出核心类型供外部模块使用
pub use address::{Address, NetLocation, ResolvedLocation};
pub use async_stream::{AsyncPing, AsyncStream};
pub use proxy_connector::ProxyConnector;
pub use stream_reader::StreamReader;
