//! 唯一跨模块协调点：重试循环。
//! 唯一同时看到 router / protocol / provider / upstream / stream 的地方。
//! 见 docs/08-mvp.md §2.2 §3.4
pub mod retry;
