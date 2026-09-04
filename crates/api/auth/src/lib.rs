//! # auth — 登录/注册/refresh/logout/self
//!
//! MVP 切片：argon2id 密码、HS256 JWT、sid+secret refresh token。
//! 不做 2FA / passkey / OAuth / 找回密码 / 邀请码 / session 30s 重放窗口。
//!
//! 表在 `auth_users` / `auth_refresh_tokens`（loose，迁移在 `migrations.rs` 启动时跑）。

pub mod ddl;
pub mod error;
pub mod jwt;
pub mod password;
pub mod routes;
pub mod service;

pub use error::AuthError;
pub use routes::{bearer_user, router};
pub use service::{AuthService, LoginResult, RefreshResult, SelfView, UserView};
