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

use axum::{Router, extract::State, http::StatusCode, response::{Json, Body}, routing::post};
use http::HeaderMap;
use serde_json::Value;
use sqlx::PgPool;

use crate::config::Config;
use crate::auth::routes::bearer_user;
use crate::auth::routes::ADMIN_ROLE_THRESHOLD;
use crate::snapshot::Snapshots;

pub mod config;
pub mod snapshot;
pub mod tavern;
pub mod usage;

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
use crate::auth::AuthService;

/// 组装完整应用 Router：admin-api + tavern + pipeline gateway + 用量中间件 + reload。
pub async fn build_app(pool: PgPool, _cfg: &Config) -> anyhow::Result<Router> {
    let egress: Arc<dyn forward::egress::Egress> = Arc::new(ReqwestEgress::new());
    assemble(pool, egress).await
}

/// 组装完整应用 Router 的公共实现。
///
/// `egress` 可注入（e2e 传 mock），生产路径由 [`build_app`] 传入 `ReqwestEgress`。
async fn assemble(
    pool: PgPool,
    egress: Arc<dyn forward::egress::Egress>,
) -> anyhow::Result<Router> {
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
        .push(GrayListGate::new(Arc::new(
            arc_swap::ArcSwap::from_pointee(gateway_gate::graylist::GrayListState::default()),
        )));

    let adaptors = Arc::new(AdaptorRegistry::with_defaults());
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

    // reload 路由（包含 admin 守卫）
    let auth_svc = Arc::new(AuthService::new(
        pool.clone(),
        std::env::var("FERRITE_JWT_SECRET")
            .unwrap_or_else(|_| "dev_ferrite_jwt_secret_key_32bytes_len!".into())
            .into_bytes(),
    )?);
    let reload_state = ReloadState {
        pool: pool.clone(),
        snapshots: Arc::new(snapshots),
        dispatcher: Arc::new(dispatcher),
        auth: auth_svc,
    };
    let reload = build_reload_router(reload_state);

    let fallback_guard = axum::middleware::from_fn(|req, next| async move {
        let path = req.uri().path();
        if path.starts_with("/v1") || path.starts_with("/v1beta") || path == "/healthz" {
            Ok(next.run(req).await)
        } else {
            Err((StatusCode::NOT_FOUND, Body::from("not found")))
        }
    });

    let pipeline_router = gateway_pipeline::router::build_router(pipeline)
        .layer(axum::middleware::from_fn_with_state(usage_state, usage::usage_middleware))
        .layer(fallback_guard);

    Ok(admin.merge(tavern).merge(reload).merge(pipeline_router))
}

/// 测试用：注入 mock egress 构建 Router（不发起真实上游请求）。
pub async fn build_app_with_egress(
    pool: PgPool,
    egress: std::sync::Arc<dyn forward::egress::Egress>,
) -> anyhow::Result<Router> {
    assemble(pool, egress).await
}

/// 快照热重载状态
///
/// 包含数据库连接、快照句柄、dispatcher 和 AuthService 实例
/// 与 assemble 中共享同一批 ArcSwap 句柄，实现热更
#[derive(Debug, Clone)]
pub struct ReloadState {
    pool: PgPool,
    snapshots: Arc<Snapshots>,
    dispatcher: Arc<Dispatcher>,
    auth: Arc<AuthService>,
}

/// 构建 reload 路由（含 admin 守卫）
pub fn build_reload_router(state: ReloadState) -> Router {
    Router::new()
        .route("/api/gateway/reload", post(reload_handler))
        .with_state(state)
}

/// 快照热重载端点（替换 501 占位）
///
/// # 语义
/// - 使用与 assemble 中共享的 `Snapshots` ArcSwap 句柄，实现热更，无需重建 gates。
/// - 使用与 admin_router 相同的 AuthService 实例（通过环境变量 `FERRITE_JWT_SECRET` 注入），确保一致的鉴权逻辑。
/// - 仅允许角色 >= 10 的管理员调用（`ADMIN_ROLE_THRESHOLD`）。
///
/// # 错误处理
/// - 401：未提供有效 Bearer token 或 token 无效
/// - 403：角色不足
/// - 500：快照加载失败或其他内部错误
///
/// # 响应
/// 返回成功状态及数据统计（tokens、users、channels 计数），方便观察热更效果。
pub async fn reload_handler(
    State(state): State<ReloadState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 校验 Bearer token + 角色
    let user = bearer_user(&state.auth, &headers)
        .map_err(|e| {
            let mut body = serde_json::json!({"success": false, "message": "未授权"});
            body["detail"] = serde_json::Value::String(e.to_string());
            (StatusCode::UNAUTHORIZED, Json(body))
        })?;

    if user.role < ADMIN_ROLE_THRESHOLD {
        let mut body = serde_json::json!({"success": false, "message": "无权限"});
        body["detail"] = serde_json::Value::String("需要管理员及以上权限".to_string());
        return Err((StatusCode::FORBIDDEN, Json(body)));
    }

    // 重新加载快照
    let new = crate::snapshot::load_snapshots(&state.pool)
        .map_err(|e| {
            let mut body = serde_json::json!({"success": false, "message": "重载失败"});
            body["detail"] = serde_json::Value::String(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, Json(body))
        })?;

    // 热更新快照数据
    state.snapshots.token_snapshot.store(new.token_snapshot.load_full());
    state.snapshots.user_snapshot.store(new.user_snapshot.load_full());
    state.snapshots.quota_snapshot.store(new.quota_snapshot.load_full());
    state.dispatcher.set_snapshot(Arc::new(new.dispatch));

    let body = serde_json::json!({
        "success": true,
        "message": "快照重载成功",
        "data": {
            "tokens": new.token_snapshot.load_full().by_hash.len(),
            "users": new.user_snapshot.load_full().by_key.len(),
            "channels": new.dispatch.channels.len(),
        }
    });

    Ok(Json(body))
}