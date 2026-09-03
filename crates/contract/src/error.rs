//! # error — 跨端共享错误码表
//!
//! 单一错误码词汇表: gateway 错误体、console 信封 message、web 前端 i18n
//! 三方共用。对齐 new-api relaykit types/channel_error.go 的归类思路。
//!
//! ## 错误码 → HTTP 状态 → 客户端行为
//!
//! | code                    | HTTP | 客户端行为 |
//! |-------------------------|------|-----------|
//! | INVALID_API_KEY         | 401  | 停止重试, 重新鉴权 |
//! | TOKEN_EXPIRED           | 401  | 刷新后重试一次 |
//! | INSUFFICIENT_QUOTA      | 402  | 引导充值 |
//! | MODEL_FORBIDDEN         | 403  | 停止重试 |
//! | RATE_LIMITED            | 429  | 指数退避重试 |
//! | NO_CANDIDATE            | 404  | 模型/分组无可用路由 |
//! | UPSTREAM_ERROR          | 502  | 网关已重试, 上游仍失败 |
//! | CATALOG_NOT_READY       | 503  | 稍后重试 (启动期) |
//! | INTERNAL                | 500  | 联系管理员 |

/// 错误码常量 — serde 序列化为字符串。
pub mod code {
    pub const INVALID_API_KEY: &str = "invalid_api_key";
    pub const TOKEN_EXPIRED: &str = "token_expired";
    pub const INSUFFICIENT_QUOTA: &str = "insufficient_quota";
    pub const MODEL_FORBIDDEN: &str = "model_forbidden";
    pub const RATE_LIMITED: &str = "rate_limited";
    pub const NO_CANDIDATE: &str = "no_candidate";
    pub const UPSTREAM_ERROR: &str = "upstream_error";
    pub const CATALOG_NOT_READY: &str = "catalog_not_ready";
    pub const INTERNAL: &str = "internal";
}

/// gateway /v1 错误体 (OpenAI 风格, 兼容主流 SDK 解析)。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayErrorBody {
    pub error: GatewayErrorDetail,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayErrorDetail {
    /// contract::error::code 常量之一。
    pub code: String,
    /// 已掩码的人类可读信息 (禁止包含上游 key/内部地址)。
    pub message: String,
    /// OpenAI 兼容字段 (SDK 读取)。
    pub r#type: String,
}

impl GatewayErrorBody {
    /// TODO(#214): 与 protocol::NormalizedError 合并 — 一个构造函数产出两种形状
    /// (OpenAI 风格 body + console Envelope), 错误码单一来源。
    pub fn from_code(_code: &str, _message: impl Into<String>) -> Self {
        todo!("TODO(#214): 定型 OpenAI 错误体映射")
    }
}
