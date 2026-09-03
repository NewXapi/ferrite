//! `gateway-protocol-bridge` —— 数据面"协议出口" Stage
//!
//! 把上游响应转换为客户端期望的协议（OpenAI / Anthropic / Gemini）。
//!
//! ## 与 `crates/protocol` 的边界
//!
//! | crate | 角色 |
//! |-------|------|
//! | `crates/protocol` | 纯函数 codec 库 (结构体 ↔ 结构体) |
//! | `gateway-protocol-bridge` | gateway 内部 stage (调 protocol, 不知道 HTTP) |
//!
//! ## 职责
//!
//! - 把 `ctx.upstream` (Forward 写入的原始上游响应) 通过 `CodecRegistry` 转换为客户端协议
//! - 把 `NormalizedError` 转换为各协议错误形状
//! - 流式响应的事件级重组（不做 SSE 帧扫描，那是 `gateway-forward` 职责）

#![doc = include_str!("README.md")]

pub mod codec;
pub mod error_mapping;
pub mod stage;

pub use codec::CodecRegistry;
pub use error_mapping::map_error;
pub use stage::ProtocolBridgeStage;
