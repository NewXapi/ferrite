//! 配置：TOML 解析、校验、`env:` 展开。叶子 crate，不依赖任何内部 crate。
//! 见 docs/08-mvp.md §1.1
pub mod error;
pub mod load;
pub mod validate;

pub use error::ConfigError;
