//! 格式转换：纯函数、零 IO。禁止依赖 upstream / config / usage。
//! 见 docs/08-mvp.md §1.4
pub mod convert;
pub mod dto;
pub mod format;
pub mod passthrough;

pub use format::Format;
