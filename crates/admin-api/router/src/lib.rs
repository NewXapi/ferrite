//! admin-api 路由聚合 — apps/api 一次性挂载。

use axum::Router;
use sqlx::PgPool;

/// 启动时建表 + 聚合 admin-api 子域 Router。
/// apps/api main.rs: `let admin = admin_api_router::router(pool).await?;`
/// DDL 失败或 FERRITE_JWT_SECRET 缺失返回 Err，由调用方决定日志/退出策略。
pub async fn router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    auth::ddl::run(&pool).await?;
    catalog::tokens::ensure_table(&pool).await?;
    tracing::info!("admin-api tables ensured");

    let secret = std::env::var("FERRITE_JWT_SECRET")
        .map_err(|_| "FERRITE_JWT_SECRET env var required")?;
    let auth_svc = std::sync::Arc::new(auth::AuthService::new(pool.clone(), secret.into_bytes()));

    let auth_router = auth::routes::router_with_svc(auth_svc.clone())?;
    let token_router = catalog::tokens::router(catalog::tokens::TokenAppState {
        svc: std::sync::Arc::new(catalog::tokens::TokenService::new(pool)),
        auth: auth_svc,
    });

    Ok(auth_router.merge(token_router))
}
