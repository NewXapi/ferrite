//! # api — Web/Console REST 契约 (前端 ↔ apps/console)
//!
//! ## 子模块地图
//!
//! | 模块 | 端点域 |
//! |------|--------|
//! | [`auth`]   | 登录/会话/刷新 |
//! | [`user`]   | 用户自读/自改 |
//! | [`token`]  | 密钥管理 |
//! | [`usage`]  | 用量查询 |
//! | [`admin`]  | 管理面: 渠道/路由/用户/分组 CRUD |
//!
//! 统一信封 [`Envelope`] 与 web 端 `client::Envelope` 同构。

pub mod admin;
pub mod auth;
pub mod token;
pub mod usage;
pub mod user;

use serde::{Deserialize, Serialize};

/// 统一响应信封 — 与 web 端 `client::Envelope` 同构。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

impl<T> Envelope<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            message: "ok".into(),
            data: Some(data),
        }
    }
    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
        }
    }
}
