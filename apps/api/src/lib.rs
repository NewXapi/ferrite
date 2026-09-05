//! Ferrite — API gateway 核心逻辑
//! 分离为 lib crate 让集成测试可以访问内部模块
//!
//! # 组装入口
//!
//! 单进程内同时挂载 admin-api 聚合路由、酒馆域路由、数据面 pipeline gateway
//! 与用量记录中间件。
//!
//! ```rust
//! use api::{build_router, config::Config};
//! use sqlx::PgPool;
//!
//! let pool = config::init_pool(&cfg.database_url).await?;
//! let router = build_router(pool, &cfg).await?;
//! ```
pub mod config;
pub mod snapshot;
pub mod tavern;
pub mod usage;

use std::sync::Arc;
use axum::Router;
use sqlx::PgPool;
use crate::config::Config;

/// 组装完整应用 Router：admin-api + tavern + pipeline gateway + 用量中间件 + reload
pub async fn build_router(pool: PgPool, cfg: &Config) -> anyhow::Result<Router> {
    // 由 main.rs 调用，具体组装逻辑在 main.rs 中复用该签名
    // 这里仅提供公共 API 供 e2e 测试复用
    unimplemented!("build_router implementation lives in main.rs; this is the public re-export signature")
}
