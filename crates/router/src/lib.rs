//! 渠道选择：倒排索引、平滑加权轮询、熔断。禁止依赖 upstream。
//! 见 docs/08-mvp.md §1.3 §4.2 §4.4
pub mod alias;
pub mod breaker;
pub mod pick;
pub mod snapshot;

pub use snapshot::Snapshot;
