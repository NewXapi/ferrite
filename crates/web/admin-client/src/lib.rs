//! client — 共享的 New API 后端 HTTP 客户端。
//!
//! 同源请求,自动注入 `Authorization: Bearer`,401 静默刷新一次,
//! 镜像 React axios client 契约。

mod manage_auth_token;
mod setup_client;
mod wire;

#[doc(hidden)]
pub use manage_auth_token::AuthState;
pub use manage_auth_token::{Refresher, TokenFuture};
pub use setup_client::ApiClient;
pub use wire::fetch_gateway_health;
pub use wire::{
    AffiliateOverviewResponse, AffiliateOverviewView, OpenTopupRequest, RedeemRequest, TopupOrder,
    UserBalanceDto, WalletResponse, WalletView, fetch_affiliate_overview, fetch_wallet, open_topup,
    redeem_code,
};
pub use wire::{
    GatewayHealthItem, GatewayHealthView, HealthItemState, InviteeView, ListEnvelope,
    TopupOrderView, fetch_invitees, fetch_topup_orders,
};

use serde::Deserialize;

/// 后端响应信封:所有 `/api` 端点回答
/// `{"success": bool, "message": str, "data": ...}`。
#[derive(Debug, Deserialize)]
pub struct Envelope<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

pub type ApiResult<T> = Result<T, ApiError>;

/// 请求失败的所有可能方式。
#[derive(Debug)]
pub enum ApiError {
    /// 网络层失败 (fetch 被拒、CORS、断网)。
    Transport(String),
    /// 非 2xx HTTP 状态码 (已恢复的 401 除外)。
    Http { status: u16, message: String },
    /// 信封到达但 `success == false`。
    Business(String),
    /// 401 且刷新无法恢复 (或未注册 refresher)。
    Unauthorized,
    /// 响应体无法解码为期望类型。
    Decode(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Transport(s) => write!(f, "{s}"),
            ApiError::Http { status, message } => write!(f, "HTTP {status}: {message}"),
            ApiError::Business(s) => write!(f, "{s}"),
            ApiError::Unauthorized => write!(f, "session expired"),
            ApiError::Decode(s) => write!(f, "decode error: {s}"),
        }
    }
}

impl std::error::Error for ApiError {}
