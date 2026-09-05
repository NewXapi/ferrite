//! `harness-tokenizer` — 真实 tokenizer（替代字符/4 guesstimate）。
//!
//! 照搬 SillyTavern `src/endpoints/tokenizers.js` 的模型加载策略：
//! HF `tokenizers` JSON（claude / llama3 等）直接 `Tokenizer::from_file`；
//! sentencepiece `.model`（llama/gemma/mistral 等）需运行时转换（暂不支持，
//! 见 [`TokenModelError::UnsupportedFormat`]，接入时再补 `sentencepiece` 依赖）。
//!
//! 零 IO 默认：模型文件路径由调用方传入（归档分发与许可核对归仓库层）。

pub mod error;
pub mod engine;
pub mod registry;

// Re-export public items for use in tests and outside the crate
pub use error::TokenModelError;
pub use error::TokenizerError;
pub use engine::TokenizerEngine;
pub use registry::TokenizerRegistry;
