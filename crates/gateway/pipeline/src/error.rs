//! `error` —— 跨 stage 错误 re-export
//!
//! 历史兼容路径：早期设计把 StageError 挂在 `gateway_pipeline::error`，
//! 消费方（protocol-bridge 等）按此路径引用。现在统一从 `stage` 导出，
//! 这里只做转发，避免改消费方代码。

pub use crate::stage::{StageError, UpstreamError};
