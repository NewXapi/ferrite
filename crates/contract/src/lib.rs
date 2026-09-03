//! # contract — 全 workspace 逻辑契约层
//!
//! 单一事实来源 (single source of truth)。center PostgreSQL、edge Fjall、
//! web (wasm) 与 apps/gateway (native) 都依赖本 crate 获得相同的类型定义。
//!
//! ## 模块地图
//!
//! | 模块 | 内容 | 消费方 |
//! |------|------|--------|
//! | [`api`]       | 前端 ↔ console 的 REST DTO (auth/user/token/usage/admin) | web, console |
//! | [`records`]   | 领域实体 (channel/routing/identity/usage/billing 子模块) | 全部 |
//! | [`mutations`] | 增量同步: MutationId / Cursor / 版本摘要 | sync, store, gateway |
//! | [`schema`]    | schema 版本常量、兼容默认值规则、fixtures | 全部 |
//! | [`error`]     | 跨端错误码表 + 网关错误体 | gateway, console, web |
//!
//! ## 铁律 (违反 = PR 拒绝)
//!
//! 1. 本 crate 不得依赖任何 runtime (tokio/sqlx/axum/reqwest/dioxus);
//! 2. 不放 SQL DDL (→ service/store/migrations) 与 Fjall key encoding (→ store);
//! 3. Web DTO 从 records 转换而来 (From 实现在本 crate);
//! 4. 新字段必须给兼容默认值 (`#[serde(default)]`); 删字段走停写窗口。

pub mod api;
pub mod error;
pub mod mutations;
pub mod records;
pub mod schema;

/// 契约 schema 版本。字段级不兼容变更时 bump, 并在 schema.rs 中登记迁移规则。
pub const SCHEMA_VERSION: u32 = 1;
