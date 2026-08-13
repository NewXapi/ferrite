//! 上游适配：纯函数、零 IO。禁止依赖 upstream / config / usage。
//! 见 docs/08-mvp.md §1.5
pub mod error;
pub mod ollama;
pub mod openai;
pub mod openai_compat;
pub mod registry;

pub use error::ErrorKind;
