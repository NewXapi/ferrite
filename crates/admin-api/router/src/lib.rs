//! admin-api 路由聚合 — apps/api 一次性挂载。

use axum::Router;
use sqlx::PgPool;

/// 启动时建表 + 返 axum Router。
/// apps/api main.rs: `let admin = admin_api_router::router(pool).await?;`
/// DDL 失败或 FERRITE_JWT_SECRET 缺失返回 Err，由调用方决定日志/退出策略。
pub async fn router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    auth::ddl::run(&pool).await?;
    tracing::info!("auth tables ensured");
    Ok(auth::router(pool)?)
}
