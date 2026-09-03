//! `harness-prompt` — 把前端物化好的 OpenAI-compatible prompt 转成 Rust 侧
//! 可序列化、可裁剪的内部类型；纯数据，不依赖 runtime / repository /
//! HTTP client；`wasm32-unknown-unknown` 友好。
//!
//! ## 模块
//!
//! - [`types`] — `AgentModelMessage` / `AgentModelContentPart` / `AgentModelRequest` / `PromptInput`
//! - [`prompt_snapshot`] — Rust 端边界；校验 `_ferrite_agent_prompt_marker`，从 OpenAI JSON
//!   反序列化成内部消息类型
//! - [`variables`] — `expand_variables`，**只**展开 `{{char}}` / `{{user}}`（前端先做宏展开，
//!   本函数为兜底；任何未识别的 `{{...}}` 保持原样，由前端再次物化）
//! - [`render`] — `PromptInput::render`，按固定顺序拼装最终 messages
//! - [`truncate`] — `truncate_history`，从最新到最旧按 token budget 裁剪，system 与
//!   tool-call/result 组原子保留
//! - [`reasoning`] — `ReasoningTemplate` + `wrap_reasoning` + `inject_reasoning`，
//!   把模型推理回填为额外 system 消息（无 UI）

//! ## 设计边界
//!
//! 宏引擎（`{{time}}` / `/roll` / `setvar` 等）在前端（Dioxus）执行；Rust 侧只接受
//! 已物化的 snapshot，并拒绝任何带 `_ferrite_agent_prompt_marker` 的 payload。
//!
//! ## 验证
//!
//! ```text
//! cargo test -p harness-prompt
//! cargo check --target wasm32-unknown-unknown -p harness-prompt
//! ```

#![deny(missing_docs)]

pub mod prompt_snapshot;
pub mod reasoning;
pub mod render;
pub mod truncate;
pub mod types;
pub mod variables;

pub use prompt_snapshot::{
    AGENT_PROMPT_MARKER_FIELD, AgentModelSnapshotKind, LEGACY_TAURITAVERN_PROMPT_MARKER_FIELD,
    PromptSnapshotError, is_prompt_snapshot, messages_from_payload, reject_unfinalized_snapshot,
    snapshot_kind, validate_prompt_snapshot_context_policy,
};
pub use reasoning::{ReasoningTemplate, inject_reasoning, wrap_reasoning};
pub use render::{RenderError, render};
pub use truncate::{TruncationDropReason, truncate_history};
pub use types::{
    AgentModelContentPart, AgentModelMessage, AgentModelRequest, AgentModelRole, PromptInput,
};
pub use variables::{VariableContext, expand_variables};
