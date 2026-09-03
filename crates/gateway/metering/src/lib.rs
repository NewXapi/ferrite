//! # metering — 计量 (热路径旁路)
//!
//! 三个子职责, 全部围绕一条原则: **转发路径上只做本地内存操作**。
//!
//! ```text
//! ① prehold  预扣 (ledger)   admission 通过后, 按预估成本扣本地余额位
//! ② scan     流扫描 (scanner) SSE 管道旁挂, 边透传边抓 usage/计数
//! ③ settle   结算 (settle)   请求结束 → 差额回补 (内存) + UsageEvent → WAL (store)
//! ```
//!
//! 定价在 [`pricing`]; 无上游 usage 时的估算在 [`estimate`]。
//! WAL 推送 center 由 service/sync 驱动; 本模块只管"产生正确的事件"。
//!
//! 崩溃语义 (明示取舍): 预扣只存内存, 崩溃即丢 — 可接受, 因为 UsageEvent
//! (WAL) 才是权威账单来源; 余额以 center 收敛后的 used_quota 为准。

pub mod estimate;
pub mod ledger;
pub mod pricing;
pub mod scanner;
pub mod settle;

pub use ledger::{Hold, Ledger};
pub use scanner::StreamScanner;
pub use settle::settle_event;
