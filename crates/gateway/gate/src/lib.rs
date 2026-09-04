//! `gateway-gate` —— 过滤层（auth / state / quota / ratelimit / model / graylist / concurrency）
//!
//! 请求主链路的**过滤层**，按 Tower / Axum 中间件的洋葱模型设计。
//! 任何 gate 失败 → `ShortCircuit(4xx)`，不进入 dispatch。
//!
//! ## 模块地图（七道闸, 一次调用全部过完）
//!
//! | 模块 | 闸门 | 失败码 |
//! |------|------|--------|
//! | [`auth`]        | ① key 哈希查表 + auth_version | INVALID_API_KEY / TOKEN_EXPIRED |
//! | [`state`]       | ② enabled / 过期 / IP 白名单 / 用户禁用 | USER_DISABLED / IP_NOT_ALLOWED |
//! | [`quota`]       | ③ 余额 vs 预估成本 | INSUFFICIENT_QUOTA |
//! | [`ratelimit`]   | ④ per-key / per-channel RPM/TPM | RATE_LIMITED |
//! | [`model`]       | ⑤ token.allowed_models 白名单 | MODEL_FORBIDDEN |
//! | [`graylist`]    | ⑥ 连续失败封禁 | GRAYLISTED |
//! | [`concurrency`] | ⑦ post-dispatch，每 channel Semaphore | CONCURRENCY_EXHAUSTED |
//! | [`snapshot`]    | (数据源) identity / quota / ip-policy 持有 | CATALOG_NOT_READY |
//!
//! ```text
//! 请求 → auth → state → quota → ratelimit → model → graylist
//!                                              ↓ (post-dispatch)
//!                                       concurrency
//!          ↑ 全部查本地内存快照, 零跨网络调用
//! ```

pub mod auth;
pub mod chain;
pub mod concurrency;
pub mod error;
pub mod graylist;
pub mod model;
pub mod quota;
pub mod ratelimit;
pub mod snapshot;
pub mod state;

pub use auth::{AuthGate, sha256};
pub use chain::{Gate, GateChain, GateCtx, Gated};
pub use concurrency::{ConcurrencyGate, ConcurrencyState};
pub use error::{Rejection, rejection_to_response};
pub use graylist as graylist_mod;
pub use graylist::{
    BLOCK_DURATION, FAIL_STREAK_THRESHOLD, FailEntry, GrayListGate, GrayListState, STREAK_WINDOW,
};
pub use model::ModelGate;
pub use quota::QuotaGate;
pub use ratelimit::{LimitScope, RateLimitGate, RateLimiter};
pub use state::StateGate;

// snapshot re-exports
pub use snapshot::{
    IpPolicy, PriceRow, PricingSnapshot, QuotaSnapshot, SharedIpPolicy, SharedPricing, SharedQuota,
    SharedTokenSnapshot, SharedUserSnapshot, TokenEntry, TokenSnapshot, UserSnapshot,
};

/// 鉴权产出的 token 元数据（chain 内部填充，最终提升到 RequestCtx.token）
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub id: i64,
    pub user_id: i64,
    pub id_hash: [u8; 32], // sha256(raw_key)
    pub group: String,
    pub enabled: bool,
    pub expires_at: Option<i64>,
    pub allowed_models: Option<Vec<String>>,
    pub auth_version: u64,
}

/// 鉴权产出的 user 元数据
#[derive(Debug, Clone)]
pub struct UserInfo {
    pub id: i64,
    pub enabled: bool,
    pub group: String,
    pub auth_version: u64,
}
