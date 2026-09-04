//! `gateway-protocol-bridge` —— 数据面协议适配层
//!
//! 厂商协议兼容的集中地。对标 new-api `relay/channel/*/adaptor.go`：
//! 每个厂商一个适配器，负责 `客户端协议 ↔ 厂商协议` 的双向转换（接口兼容）。
//!
//! ## 职责
//!
//! - [`adaptor`] — 厂商适配器注册表：Codec trait（请求/响应字节转换）+ 厂商实现
//! - [`sse`] — SSE 帧扫描（事件边界 / keepalive / 终止），源自原 protocol crate
//! - [`error_mapping`] — `contract::error::NormalizedError` → 各协议错误形状
//! - [`stage`] — pipeline Stage 4：把上游响应经适配器转为客户端协议
//!
//! ## 与其他 crate 的边界
//!
//! | crate | 角色 |
//! |-------|------|
//! | `gateway-forward` | IO 编排（发上游/流回传），不碰协议转换 |
//! | `gateway-protocol-bridge` | 协议转换（纯函数，无 IO） |
//! | `contract::error::NormalizedError` | 跨 crate 单一错误协议 |

pub mod adaptor;
pub mod error_mapping;
pub mod sse;
pub mod stage;

pub use adaptor::{AdaptorRegistry, Codec, Protocol};
pub use error_mapping::map_error;
pub use stage::ProtocolBridgeStage;
