//! client — shared HTTP client for the New API backend.
//!
//! Same-origin requests with automatic `Authorization: Bearer` injection and
//! one-shot 401 token refresh, mirroring the React axios client contract.

mod manage_auth_token;
mod setup_client;

pub use manage_auth_token::{Refresher, TokenFuture};
pub use setup_client::ApiClient;

use serde::Deserialize;

/// Backend response envelope: every `/api` endpoint answers
/// `{"success": bool, "message": str, "data": ...}`.
#[derive(Debug, Deserialize)]
pub struct Envelope<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

pub type ApiResult<T> = Result<T, ApiError>;

/// Every way a request can fail.
#[derive(Debug)]
pub enum ApiError {
    /// Network-level failure (fetch rejected, CORS, offline).
    Transport(String),
    /// Non-2xx HTTP status (except recovered 401s).
    Http { status: u16, message: String },
    /// Envelope arrived but `success == false`.
    Business(String),
    /// 401 that refresh could not recover (or no refresher registered).
    Unauthorized,
    /// Body could not be decoded into the expected type.
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn envelope_success_with_data() {
        let json = r#"{"success":true,"message":"ok","data":42}"#;
        let env: Envelope<i32> = serde_json::from_str(json).unwrap();
        assert!(env.success);
        assert_eq!(env.message, "ok");
        assert_eq!(env.data, Some(42));
    }

    #[test]
    fn envelope_success_false() {
        let json = r#"{"success":false,"message":"error","data":null}"#;
        let env: Envelope<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(!env.success);
        assert_eq!(env.message, "error");
    }

    #[test]
    fn envelope_null_data_with_unit() {
        let json = r#"{"success":true,"message":"ok","data":null}"#;
        let env: Envelope<()> = serde_json::from_str(json).unwrap();
        assert!(env.success);
        assert_eq!(env.data, None);
    }
}
