//! newapi-session — login state for the New API frontend.
//!
//! Owns the authenticated identity (user + access token + session id), the
//! login/2FA/refresh/logout flows, and the global signal every page reads.

pub mod login;
pub mod manage_session;
pub mod refresh_token;

use serde::Deserialize;

/// Dashboard user (subset of the backend self-user DTO; unknown fields ignored).
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub display_name: String,
    pub role: i32,
    pub status: i32,
    pub email: String,
    pub group: String,
    pub quota: i64,
    pub used_quota: i64,
    pub request_count: i64,
}

/// Server login session reference; only `sid` is needed client-side
/// (sent as the `X-Auth-Session` header on refresh/logout).
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct SessionInfo {
    pub sid: String,
}

/// Full authentication bundle returned by login / 2FA verify / refresh.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct AuthBundle {
    pub access_token: String,
    pub token_type: String,
    pub access_expires_at: i64,
    pub session: SessionInfo,
    pub user: User,
}

pub use login::{login, verify_2fa, LoginOutcome};
pub use manage_session::{clear_session, init, logout, SessionState, SESSION};
pub use refresh_token::refresh;
