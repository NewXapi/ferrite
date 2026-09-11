//! DB bootstrap — sqlx 迁移的单一权威入口。
//!
//! 生产装配（admin-router::router）与集成测试的建表一律走本 crate：
//! `db/migrations/` 是唯一的 schema 事实源，各 crate 的 `ensure_table`
//! 补丁式建表已退役。
//!
//! 迁移文件全部幂等（IF NOT EXISTS / ON CONFLICT / DROP IF EXISTS），
//! 对"已被旧版 ensure_table 建过表的存量库"与全新库都能安全执行；
//! 已应用版本记录在 `_sqlx_migrations` 表。

use sqlx::PgPool;

/// 应用 `db/migrations/` 下全部未执行的迁移。
///
/// 路径相对本 crate manifest（`crates/api/db-bootstrap` → 仓库根 `db/migrations`）。
/// 迁移按文件名版本号顺序在事务中执行；失败即返回
/// [`sqlx::migrate::MigrateError`]，调用方决定日志/退出策略。
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("../../../db/migrations")
        .run(pool)
        .await
        .map(|_| ())
}
