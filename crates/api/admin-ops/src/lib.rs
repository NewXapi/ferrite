//! # ops — 运维域 (center)
//!
//! 单机版范围收窄：只保留运行时选项（options）。
//!
//! 任务 runner（`SKIP LOCKED` 认领 + lease 续租，TODO(#800)）、通知
//! (email/webhook/bark)、探活定时调度（ops::probe，TODO(#801)）在单机版
//! **明确不做**（用户要求：单机部署、探活手动触发即可）。原 stub 已移除，
//! 完整设计保留在 git 历史，需要时从历史恢复。
//!
//! ## 模块地图
//!
//! | 模块 | 职责 |
//! |------|------|
//! | [`options`] | 运行时选项 (key/value + 类型化校验，平表直连) |

pub mod options;

pub use options::{ensure_table, router, OptionsAppState, OptionsService};
