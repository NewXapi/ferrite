//! admin-api 路由聚合 — apps/api 一次性挂载。

use axum::Router;
use sqlx::PgPool;

/// 启动时建表 + 聚合 admin-api 子域 Router。
/// apps/api main.rs: `let admin = admin_api_router::router(pool).await?;`
/// DDL 失败或 FERRITE_JWT_SECRET 缺失返回 Err，由调用方决定日志/退出策略。
pub async fn router(pool: PgPool) -> Result<Router, Box<dyn std::error::Error>> {
    auth::ddl::run(&pool).await?;
    catalog::tokens::ensure_table(&pool).await?;
    catalog::channels::ensure_table(&pool).await?;
    catalog::groups::ensure_table(&pool).await?;
    catalog::models::ensure_table(&pool).await?;
    observe::logs::ensure_table(&pool).await?;
    observe::monitor::ensure_table(&pool).await?;
    billing::ensure_table(&pool).await?;
    ops::ensure_table(&pool).await?;
    catalog::routes::ensure_table(&pool).await?;
    tracing::info!("admin-api tables ensured");

    let secret =
        std::env::var("FERRITE_JWT_SECRET").map_err(|_| "FERRITE_JWT_SECRET env var required")?;
    let auth_svc = std::sync::Arc::new(auth::AuthService::new(pool.clone(), secret.into_bytes())?);

    let auth_router = auth::routes::router_with_svc(auth_svc.clone())?;

    let token_router = catalog::tokens::router(catalog::tokens::TokenAppState {
        svc: std::sync::Arc::new(catalog::tokens::TokenService::new(pool.clone())),
        auth: auth_svc.clone(),
    });
    let channel_router = catalog::channels::router(catalog::channels::ChannelAppState {
        svc: std::sync::Arc::new(catalog::channels::ChannelService::new(pool.clone())),
        auth: auth_svc.clone(),
        monitor: observe::monitor::MonitorDeps::new(pool.clone()),
    });
    let group_router = catalog::groups::router(catalog::groups::GroupAppState {
        svc: std::sync::Arc::new(catalog::groups::GroupService::new(pool.clone())),
        auth: auth_svc.clone(),
    });
    let model_router = catalog::models::router(catalog::models::ModelAppState {
        svc: std::sync::Arc::new(catalog::models::ModelService::new(pool.clone())),
        auth: auth_svc.clone(),
    });
    let log_router = observe::logs::router(observe::logs::LogAppState {
        svc: std::sync::Arc::new(observe::logs::LogService::new(pool.clone())),
        auth: auth_svc.clone(),
    });
    let redeem_router = billing::router(billing::RedeemAppState {
        svc: std::sync::Arc::new(billing::RedeemService::new(pool.clone())),
        auth: auth_svc.clone(),
    });
    let route_unit_router = catalog::routes::router(catalog::routes::RouteUnitAppState {
        svc: std::sync::Arc::new(catalog::routes::RouteUnitService::new(pool.clone())),
        auth: auth_svc.clone(),
    });
    let options_router = ops::router(ops::OptionsAppState {
        svc: std::sync::Arc::new(ops::OptionsService::new(pool.clone())),
        auth: auth_svc.clone(),
    });
    let monitor_router = observe::monitor::router(observe::monitor::MonitorAppState {
        deps: observe::monitor::MonitorDeps::new(pool.clone()),
        auth: auth_svc.clone(),
    });

    Ok(auth_router
        .merge(token_router)
        .merge(channel_router)
        .merge(group_router)
        .merge(model_router)
        .merge(redeem_router)
        .merge(options_router)
        .merge(route_unit_router)
        .merge(log_router)
        .merge(monitor_router))
}
