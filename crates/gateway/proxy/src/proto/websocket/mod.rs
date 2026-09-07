//! WebSocket 传输层（ported from shoes, MIT）。
//!
//! 提供 WS 帧编解码与客户端 handler，供 VMess/VLESS over WS 组合。
//! 适配 PR1 [`crate::proto::ProxyConnector`] trait。

pub mod handler;
pub mod stream;

/// 与 shoes util::allocate_vec 相同（websocket 流缓冲用）。
#[inline]
#[allow(clippy::uninit_vec)]
pub fn allocate_vec<T: Copy>(len: usize) -> Vec<T> {
    let mut ret = Vec::with_capacity(len);
    unsafe {
        ret.set_len(len);
    }
    ret
}

pub use handler::{WebsocketServerTarget, WebsocketTcpClientHandler};
pub use stream::WebsocketStream;

/// WS ping 类型（shoes config::WebsocketPingType 等价）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WebsocketPingType {
    #[default]
    None,
    EmptyFrame,
    PingFrame,
}
