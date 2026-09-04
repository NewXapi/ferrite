//! # shared-web — 跨应用通用的前端共享能力层
//!
//! 提供 admin-web 和 tavern-web 双端共用的：
//! - 认证弹窗 (`AuthModal`) 与会话存取 (`SessionManager`)
//! - 用户徽标 (`UserBadge`)
//! - 契约 DTO 消费与登录/注册 API 客户端

pub mod auth_modal;
pub mod session;

pub use auth_modal::{AuthModal, UserBadge};
pub use session::{
    api_login, api_register, clear_cached_session, get_cached_token, get_cached_user,
    set_cached_session,
};
