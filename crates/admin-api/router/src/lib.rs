//! admin-api 路由聚合 — apps/api 一次性挂载。

use axum::Router;
use sqlx::PgPool;

/// 启动时建表 + 返 axum Router。
/// apps/api 在 main.rs 里调 `admin_api_router::router(pool).merge(gateway_router)`。
pub async fn router(pool: PgPool) -> Router {
    auth::ddl::run(&pool)
        .await
        .expect("failed to run auth ddl");
    tracing::info!("auth tables ensured");
    auth::router(pool)
}
