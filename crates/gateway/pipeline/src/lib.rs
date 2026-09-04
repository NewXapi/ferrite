//! `gateway-pipeline` —— gateway 编排核心
//!
//! 定义 Stage trait / RequestCtx / Pipeline / axum 集成，是各 gateway crate
//! （gate / dispatch / forward / protocol-bridge / proxy / security）的公共依赖。
//! 反向不依赖任何具体 stage crate，保持依赖方向单向。
//!
//! ## 文件分工
//!
//! - [`ctx`] —— 请求上下文: `RequestCtx` / `RequestMeta` / `BodySource` / `PipeStream` / `ProtocolKind`
//! - [`stage`] —— `Stage` trait + `StageOutcome` + `StageError` / `UpstreamError`
//! - [`pipeline`] —— 链式 `Pipeline` 编排器
//! - [`router`] —— axum 集成 (`build_router` + `error_to_response`)
//! - [`error`] —— 历史兼容路径 re-export（`gateway_pipeline::error::StageError`）
//!
//! 共享契约类型只保留跨 crate 必需的 [`TokenInfo`]；其余快照（token / user /
//! pricing / quota / ip-policy / sensitive-words）由各自持有方实现：
//! - `gate` 的 `snapshot.rs`（TokenSnapshot / UserSnapshot 已在其内）
//! - `gate` 的 `quota.rs` / `state.rs` / `ratelimit.rs`
//! - `security` 的 `wordlist.rs`
//! - `dispatch` 的 `health.rs` / `ratelimit.rs`
//! - `metering` 的 `ledger.rs` / `pricing.rs`

pub mod ctx;
pub mod error;
pub mod pipeline;
pub mod router;
pub mod stage;

pub use ctx::{BodySource, PipeStream, ProtocolKind, RequestCtx, RequestMeta, SelectedRoute};
pub use error::{StageError, UpstreamError};
pub use pipeline::Pipeline;
pub use router::{build_router, error_to_response};
pub use stage::{Stage, StageOutcome};

/// Token 鉴权后产物（Admission 写入 RequestCtx.token）
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub id: i64,
    pub group: String,
    pub enabled: bool,
    pub allowed_models: Option<Vec<String>>,
    pub auth_version: u64,
}
