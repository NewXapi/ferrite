//! Ferrite — API gateway 核心逻辑
//!
//! 分离为 lib crate 让集成测试可以访问内部模块。
//!
//! # 组装入口
//!
//! ```rust
//! use api::{build_app, config::Config};
//! use sqlx::PgPool;
//!
//! # async fn example() -> anyhow::Result<()> {
//! # let cfg = Config { database_url: String::new(), listen: "0.0.0.0:3000".into(), log_level: "info".into() };
//! # let pool = sqlx::PgPool::connect("postgres://localhost/ferrite").await?;
//! let router = build_app(pool, &cfg).await?;
//! # Ok(())
//! } ```

use std::sync::Arc;

use axum::Router;
use sqlx::PgPool;

use crate::config::Config;

pub mod config;
pub mod snapshot;
pub mod tavern;
pub mod usage;

use admin_router::router as admin_router;
use dispatch::stage::DispatchStage;
use dispatch::{Dispatcher, MemoryHealthTable};
use forward::egress::ReqwestEgress;
use forward::stage::ForwardStage;
use gateway_gate::auth::AuthGate;
use gateway_gate::chain::GateChain;
use gateway_gate::graylist::GrayListGate;
use gateway_gate::model::ModelGate;
use gateway_gate::quota::QuotaGate;
use gateway_gate::ratelimit::{RateLimitGate, RateLimiter};
use gateway_gate::snapshot::{IpPolicy, PricingSnapshot};
use gateway_gate::state::StateGate;
use gateway_pipeline::pipeline::Pipeline;
use gateway_protocol_bridge::adaptor::AdaptorRegistry;
use gateway_protocol_bridge::stage::ProtocolBridgeStage;

/// 组装完整应用 Router：admin-api + tavern + pipeline gateway + 用量中间件 + reload。
pub async fn build_app(pool: PgPool, _cfg: &Config) -> anyhow::Result<Router> {
    // admin-api 聚合路由（内部已含 auth，不再单独挂载 auth::router）
    let admin = admin_router::router(pool.clone())
        .await
        .map_err(|e| anyhow::anyhow!("failed to initialize admin router: {e}"))?;

    // 酒馆域路由
    let tavern = tavern::router(&tavern::TavernConfig::default())?;

    // 从 PG 加载快照 → Dispatcher + gates → Pipeline
    let snapshots = snapshot::load_snapshots(&pool).await?;
    let health = Arc::new(MemoryHealthTable::new());
    let dispatcher = Arc::new(Dispatcher::new(
        Some(Arc::new(snapshots.dispatch.clone())),
        health.clone(),
    ));

    // 快照已是 Shared*（Arc<ArcSwap<T>>），直接喂给 gate
    let gates = GateChain::new()
        .push(AuthGate::new(snapshots.token_snapshot.clone()))
        .push(StateGate::new(
            snapshots.user_snapshot.clone(),
            Arc::new(arc_swap::ArcSwap::from_pointee(IpPolicy::default())),
        ))
        .push(ModelGate)
        .push(QuotaGate::new(
            snapshots.quota_snapshot.clone(),
            Arc::new(arc_swap::ArcSwap::from_pointee(PricingSnapshot::default())),
        ))
        .push(RateLimitGate::new(Arc::new(RateLimiter::new(100, 60))))
        .push(GrayListGate::new(Arc::new(arc_swap::ArcSwap::from_pointee(
            gateway_gate::graylist::GrayListState::default(),
        ))));

    let adaptors = Arc::new(AdaptorRegistry::with_defaults());
    let egress = Arc::new(ReqwestEgress::new());
    let pipeline = Arc::new(
        Pipeline::new()
            .push(gates)
            .push(DispatchStage::new(dispatcher))
            .push(ForwardStage::new(egress, adaptors.clone()))
            .push(ProtocolBridgeStage::new(adaptors)),
    );

    // 用量中间件包 pipeline router
    let usage_state = usage::UsageMiddlewareState {
        pool: pool.clone(),
        snapshots: Arc::new(snapshots),
    };
    let pipeline_router = gateway_pipeline::router::build_router(pipeline)
        .layer(axum::middleware::from_fn_with_state(
            usage_state,
            usage::usage_middleware,
        ));

    // reload 端点（501 占位；真实实现需 admin 守卫 + 热更快照）
    let reload = Router::new().route("/api/gateway/reload", axum::routing::post(reload_handler));

    // 合并：具体路由优先，pipeline 作为 fallback 兜底 /v1/*
    Ok(admin
        .merge(tavern)
        .merge(reload)
        .merge(pipeline_router))
}

async fn reload_handler() -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    // ponytail: 热重载快照；真实实现需要 admin 守卫 + 重建 gates/Dispatcher 后 store。
    // 本 PR 先用重启替代，返回 501 占位。
    (
        axum::http::StatusCode::NOT_IMPLEMENTED,
        axum::Json(serde_json::json!({
            "code": 501,
            "message": "reload not implemented; restart process to refresh snapshots"
        })),
    )
}
