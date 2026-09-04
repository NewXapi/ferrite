//! tavern-state — 酒馆前端全局状态。
//!
//! 职责：
//! - 当前角色 / 聊天 / 消息列表（含 swipes）
//! - 生成中状态与流式缓冲（[`GenerationState`]）
//! - 中止语义：置位后 [`append_delta`] 变 no-op
//!
//! 状态只依赖 [`tavern_client`] 做落盘，不直接发 HTTP。
//! 宏替换与 reasoning 注入是后续任务，不在本 crate。
