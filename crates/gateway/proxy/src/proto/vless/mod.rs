//! VLESS 客户端握手（ported from shoes, MIT）。
//!
//! 协议：VLESS 头部由用户 ID（16 字节 UUID）+ 附加数据 + 目标地址帧组成。
//!
//! 不移植：server 端、UDP ASSOCIATE、h2mux、shadowsocks-over-vless、Vision 流、Reality（超范围）。
//!
//! PR3：适配 `ProxyConnector` 协议，实现链式客户端接入。
//!
//! 对外类型名（实现即 `VlessProxyConnector`）：
//! - `VlessProxyConnector = VlessTcpClientHandler`
//!
//! 原计划中废弃的 UDP-over-TCP 见 `VlessTcpClientHandler::setup_client_udp_bidirectional`，
//! 可选启用用于 UDP 穿透。
//!

pub mod vless_client_handler;
pub mod vless_util;

pub use vless_client_handler::VlessProxyConnector;
