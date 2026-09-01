//! 准入拒绝原因 — 映射到 contract::error 错误码 + OpenAI 风格 4xx 错误体。

/// 拒绝原因 — 每个变体可直接映射 contract::error 错误码。
#[derive(Debug, Clone, thiserror::Error)]
pub enum Rejection {
    /// 无效 key (快照中不存在此 hash)。
    #[error("invalid api key")]
    InvalidKey,

    /// key 存在但被禁用/过期。携带人类可读原因。
    #[error("token unavailable: {0}")]
    TokenUnavailable(String),

    /// 余额不足。预估成本 (内部单位) 供错误信息展示。
    #[error("insufficient quota: need {estimated}, have {available}")]
    InsufficientQuota { estimated: i64, available: i64 },

    /// 该 key 的分组无权访问请求的模型 (allowed_models 白名单)。
    #[error("model {model} not allowed for group {group}")]
    ModelForbidden { model: String, group: String },

    /// 并发已满 — 客户端应退避重试 (429)。
    #[error("channel busy")]
    Busy,
}

impl Rejection {
    /// 对应 contract::error 错误码 (错误体构造的单一来源)。
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidKey => contract::error::code::INVALID_API_KEY,
            Self::TokenUnavailable(_) => contract::error::code::TOKEN_EXPIRED,
            Self::InsufficientQuota { .. } => contract::error::code::INSUFFICIENT_QUOTA,
            Self::ModelForbidden { .. } => contract::error::code::MODEL_FORBIDDEN,
            Self::Busy => contract::error::code::RATE_LIMITED,
        }
    }

    /// 对客户端暴露的 HTTP 状态码。
    pub fn http_status(&self) -> u16 {
        match self {
            Self::InvalidKey | Self::TokenUnavailable(_) => 401,
            Self::InsufficientQuota { .. } => 402,
            Self::ModelForbidden { .. } => 403,
            Self::Busy => 429,
        }
    }
}
