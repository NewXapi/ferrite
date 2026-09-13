//! admin-api 路由聚合 — apps/api 一次性挂载。
//!
//! apps/api main.rs: `let admin = admin_router::router(pool, auth_svc, proxies).await?;`
//! `auth_svc` 由 app 层组装后注入（FERRITE_JWT_SECRET 是组装关注点，本 crate 不读环境变量）。
//! DDL 失败返回 Err，由调用方决定日志/退出策略。

use billing::{
    AffiliateAppState, CurrencyAppState, TopupAppState, WalletAppState, affiliate_router,
    currency_router, topup_router, wallet_router,
};
use sqlx::PgPool;

/// 启动时建表 + 聚合 admin-api 子域 Router。
pub async fn router(
    pool: PgPool,
    auth_svc: std::sync::Arc<auth::AuthService>,
    proxies: std::sync::Arc<gateway_proxy::ProxyManager>,
) -> Result<axum::Router, Box<dyn std::error::Error>> {
    // 建表唯一入口：db/migrations（ensure_table 补丁式建表已退役）。
    db_bootstrap::run_migrations(&pool).await?;
    tracing::info!("db migrations applied");

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
    let options_router = ops::router(ops::OptionsAppState {
        svc: std::sync::Arc::new(ops::OptionsService::new(pool.clone())),
        auth: auth_svc.clone(),
    });
    let monitor_router = observe::monitor::router(observe::monitor::MonitorAppState {
        deps: observe::monitor::MonitorDeps::new(pool.clone()),
        auth: auth_svc.clone(),
    });
    let system_info_router = ops::system_info_router(ops::SystemInfoAppState {
        svc: std::sync::Arc::new(ops::SystemInfoService::new(
            pool.clone(),
            ops::ProcessTimeTracker::default(),
        )),
        auth: auth_svc.clone(),
    });
    let proxy_node_router = admin_proxy::router(admin_proxy::ProxyNodeAppState {
        svc: std::sync::Arc::new(admin_proxy::ProxyNodeService::new(pool.clone())),
        auth: auth_svc.clone(),
        proxies,
    });

    // 新增 billing 子域 router
    let wallet_svc = std::sync::Arc::new(billing::WalletService::new(pool.clone()));
    let currency_svc = std::sync::Arc::new(billing::CurrencyService::new(pool.clone()));
    let affiliate_svc = std::sync::Arc::new(billing::AffiliateService::new(
        pool.clone(),
        billing::WalletService::new(pool.clone()),
    ));
    let topup_svc = std::sync::Arc::new(billing::TopupService::new(pool.clone()));

    let wallet_router = wallet_router(WalletAppState {
        svc: wallet_svc,
        auth: auth_svc.clone(),
    });
    let currency_router = currency_router(CurrencyAppState {
        svc: currency_svc,
        auth: auth_svc.clone(),
    });
    let affiliate_router = affiliate_router(AffiliateAppState {
        svc: affiliate_svc,
        auth: auth_svc.clone(),
    });
    let topup_router = topup_router(TopupAppState {
        svc: topup_svc,
        auth: auth_svc.clone(),
    });

    // auth 子路由自身不带前缀（/login /register ...），必须 nest 到 /api/user
    // 与前端 admin-client 约定的 /api/user/{login,register,...} 对齐。
    let auth_router = axum::Router::new().nest("/api/user", auth_router);

    Ok(auth_router
        .merge(token_router)
        .merge(channel_router)
        .merge(group_router)
        .merge(model_router)
        .merge(redeem_router)
        .merge(wallet_router)
        .merge(currency_router)
        .merge(affiliate_router)
        .merge(topup_router)
        .merge(options_router)
        .merge(log_router)
        .merge(monitor_router)
        .merge(system_info_router)
        .merge(proxy_node_router))
}
