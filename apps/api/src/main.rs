//! Ferrite — 进程入口，只做组装和启动
//!
//! 单进程内挂载：
//! - admin-api 聚合路由（auth + catalog + observe）
//! - tavern 域路由
//! - pipeline gateway 数据面（/v1/* fallback）
//! - reload 端点（admin 角色）
//! - usage 中间件（拦截 /v1/* POST 记录用量）

use api::config;
use api::snapshot;
use api::tavern;
use api::usage;
use admin_router::router as admin_router;
use axum::{
    extract::{ConnectInfo, State},
    routing::{get, post},
    Router,
};
use gateway_pipeline::router as pipeline_router;
use gateway_gate::chain::GateChain;
use gateway_gate::dispatch::{Dispatcher, MemoryHealthTable, Snapshot};
use gateway_gate::snapshot::{IpPolicy, PricingSnapshot, QuotaSnapshot, TokenSnapshot, UserSnapshot};
use gateway_pipeline::pipeline::Pipeline;
use gateway_pipeline::stage::{DispatchStage, ForwardStage, ProtocolBridgeStage};
use gateway_forward::egress::ReqwestEgress;
use gateway_protocol_bridge::adaptor::AdaptorRegistry;
use std::sync::Arc;
use std::net::SocketAddr;
use std::process::ExitCode;
use anyhow::Result;
use tracing_subscriber::{Layer as _, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::rolling::daily;
use tracing_appender::non_blocking;

// LOG_DIR 常量搬进 main.rs，原名原值
const LOG_DIR: &str = "logs";

#[tokio::main]
async fn main() -> ExitCode {
    // 1. config/pool/tracing 保留
    let config = match config::load_config(std::path::Path::new("config/config.toml")) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("failed to load config: {e}");
            return ExitCode::FAILURE;
        }
    };

    let pool = match config::init_pool(&config.database_url).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("failed to connect to database: {e}");
            return ExitCode::FAILURE;
        }
    };

    // 统一遥测
    let file_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level));
    let stdout_filter = file_filter.clone();
    let (file_writer, log_guard) = non_blocking(daily(LOG_DIR, "ferrite.log"));
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(stdout_filter))
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(file_writer)
                .with_filter(file_filter),
        )
        .init();
    let _log_guard = log_guard;
    let _ = tracing_log::LogTracer::init();

    tracing::info!("database connected");

    // 2. admin_api_router::router(pool).await? 挂载（内含 auth，删除原有独立 auth::router 挂载）
    let admin = match admin_router::router(pool.clone()).await {
        Ok(r) => {
            tracing::info!("admin api router mounted at /api/*");
            r
        }
        Err(e) => {
            tracing::error!("failed to initialize admin router: {e}");
            return ExitCode::FAILURE;
        }
    };

    // 3. tavern 路由
    let tavern = match tavern::router(&tavern::TavernConfig::default()) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "failed to initialize tavern storage");
            return ExitCode::FAILURE;
        }
    };

    // 4. snapshot::load_snapshots(&pool) → Dispatcher + gates → Pipeline
    let snapshots = match snapshot::load_snapshots(&pool).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("failed to load snapshots: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Dispatcher with real dispatch snapshot
    let health = Arc::new(MemoryHealthTable::new());
    let dispatch_snapshot = Arc::new(Snapshot {
        units: snapshots.dispatch.units.clone(),
        channels: snapshots.dispatch.channels.clone(),
    });
    let dispatcher = Arc::new(Dispatcher::new(Some(dispatch_snapshot), health.clone()));

    // Gates with real Token/User/Quota snapshots from Snapshots
    let token_swap = Arc::new(arc_swap::ArcSwap::from_pointee(snapshots.token_snapshot.clone()));
    let user_swap = Arc::new(arc_swap::ArcSwap::from_pointee(snapshots.user_snapshot.clone()));
    let quota_swap = Arc::new(arc_swap::ArcSwap::from_pointee(snapshots.quota_snapshot.clone()));

    // Build gates: AuthGate, StateGate, ModelGate, QuotaGate, RateLimitGate, GrayListGate
    let gates = GateChain::new()
        .push(gateway_gate::auth::AuthGate::new(token_swap.clone()))
        .push(gateway_gate::state::StateGate::new(
            user_swap.clone(),
            Arc::new(arc_swap::ArcSwap::from_pointee(IpPolicy::default())),
        ))
        .push(gateway_gate::model::ModelGate)
        .push(gateway_gate::quota::QuotaGate::new(
            quota_swap.clone(),
            Arc::new(arc_swap::ArcSwap::from_pointee(PricingSnapshot::default())),
        ))
        .push(gateway_gate::ratelimit::RateLimitGate::new(Arc::new(
            gateway_gate::ratelimit::RateLimiter::new(100, 60),
        )))
        .push(gateway_gate::graylist::GrayListGate::new(Arc::new(
            arc_swap::ArcSwap::from_pointee(gateway_gate::graylist::GrayListState::default()),
        )));

    // Pipeline with all stages in correct order
    let adaptors = Arc::new(AdaptorRegistry::with_defaults());
    let egress = Arc::new(ReqwestEgress::new());
    let pipeline = Arc::new(
        Pipeline::new()
            .push(gates)
            .push(DispatchStage::new(dispatcher.clone()))
            .push(ForwardStage::new(egress, adaptors.clone()))
            .push(ProtocolBridgeStage::new(adaptors)),
    );

    // 5. usage 中间件包 pipeline router
    let usage_state = usage::UsageMiddlewareState {
        pool: pool.clone(),
        snapshots: Arc::new(snapshots.clone()),
        token_swap: token_swap.clone(),
        user_swap: user_swap.clone(),
        quota_swap: quota_swap.clone(),
    };
    let pipeline_router = pipeline_router::build_router(pipeline)
        .layer(axum::middleware::from_fn_with_state(
            usage_state,
            usage::usage_middleware,
        ));

    // 6. reload 路由 POST /api/gateway/reload：auth::routes::bearer_user 校验 admin 角色后
    // load_snapshots → dispatcher.set_snapshot(...) + 三个 ArcSwap .store 新值
    let reload = Router::new().route(
        "/api/gateway/reload",
        post(reload_handler).with_state(ReloadState {
            pool: pool.clone(),
            dispatcher: dispatcher.clone(),
            token_swap: token_swap.clone(),
            user_swap: user_swap.clone(),
            quota_swap: quota_swap.clone(),
        }),
    );

    // 7. merge 顺序：admin_router.merge(tavern).merge(reload).merge(pipeline_router)
    // pipeline 是 fallback，放最后
    let app = admin
        .merge(tavern)
        .merge(reload)
        .merge(pipeline_router);

    // 8. axum::serve 用 into_make_service_with_connect_info 以便中间件拿真实 IP
    let listener = match tokio::net::TcpListener::bind(&config.listen).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("failed to bind {}: {e}", config.listen);
            return ExitCode::FAILURE;
        }
    };

    tracing::info!("listening on {}", config.listen);

    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("shutdown signal received");
    };

    if let Err(e) = serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown)
        .await
    {
        tracing::error!("server error: {e}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

#[derive(Clone)]
struct ReloadState {
    pool: sqlx::PgPool,
    dispatcher: Arc<Dispatcher>,
    token_swap: Arc<arc_swap::ArcSwap<TokenSnapshot>>,
    user_swap: Arc<arc_swap::ArcSwap<UserSnapshot>>,
    quota_swap: Arc<arc_swap::ArcSwap<QuotaSnapshot>>,
}

async fn reload_handler(
    State(state): State<ReloadState>,
    ConnectInfo(_): ConnectInfo<SocketAddr>,
) -> Result<axum::Json<serde_json::Value>, axum::response::Response> {
    // admin 角色校验通过 auth::routes::bearer_user 在 admin_router 内部已做
    let snapshots = snapshot::load_snapshots(&state.pool).await
        .map_err(|e| {
            tracing::error!("reload snapshots failed: {e}");
            axum::response::Response::builder()
                .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::Body::from(format!("reload failed: {e}")))
                .unwrap()
        })?;

    // Update dispatcher snapshot
    state.dispatcher.set_snapshot(Arc::new(gateway_gate::dispatch::Snapshot {
        units: snapshots.dispatch.units.clone(),
        channels: snapshots.dispatch.channels.clone(),
    }));

    // Update three ArcSwap snapshots
    state.token_swap.store(Arc::new(snapshots.token_snapshot.clone()));
    state.user_swap.store(Arc::new(snapshots.user_snapshot.clone()));
    state.quota_swap.store(Arc::new(snapshots.quota_snapshot.clone()));

    tracing::info!("gateway snapshots reloaded");
    Ok(axum::Json(serde_json::json!({"status": "ok", "message": "snapshots reloaded"})))
}