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
    // 出口代理池：DB proxy_nodes 表（enabled）→ ProxyManager；
    // 管理台 CRUD 会原地 reload（见 admin-router /api/proxy_nodes）。
    let proxies = Arc::new(gateway_proxy::ProxyManager::new());
    proxies.install(admin_proxy::load_proxy_snapshot(&pool).await?);
    // 主动探测循环（M3-B）：默认关闭，options 表 proxy.probe_enabled=true 才开。
    spawn_probe_loop(pool.clone(), proxies.clone());

    // admin-api 聚合路由（内部已含 auth，不再单独挂载 auth::router）
    let admin = admin_router::router(pool.clone(), proxies.clone())
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
            .push(ForwardStage::new(egress, adaptors.clone()).with_proxies(proxies.clone()))
            .push(ProtocolBridgeStage::new(adaptors)),
    );

    // 用量中间件包 pipeline router
    let usage_state = usage::UsageMiddlewareState {
        pool: pool.clone(),
        snapshots: Arc::new(snapshots),
    };
    let pipeline_router = gateway_pipeline::router::build_router(pipeline).layer(
        axum::middleware::from_fn_with_state(usage_state, usage::usage_middleware),
    );

    // reload 端点（501 占位；真实实现需 admin 守卫 + 热更快照）
    let reload = Router::new().route("/api/gateway/reload", axum::routing::post(reload_handler));

    // 合并：具体路由优先，pipeline 作为 fallback 兜底 /v1/*
    Ok(admin.merge(tavern).merge(reload).merge(pipeline_router))
}

/// 测试用：注入 mock egress 构建 Router（不发起真实上游请求）。
pub async fn build_app_with_egress(
    pool: PgPool,
    egress: std::sync::Arc<dyn forward::egress::Egress>,
) -> anyhow::Result<Router> {
    assemble(pool, egress).await
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

// ---------- M3-B 主动探测循环 ----------

/// options 表的探测相关键。默认都没有 → 探测关闭。
mod probe_keys {
    pub const ENABLED: &str = "proxy.probe_enabled";
    pub const INTERVAL_SECS: &str = "proxy.probe_interval_secs";
}

/// options 行的探测配置（纯数据，便于单测 JSONB 解析缺省逻辑）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeOptions {
    pub enabled: bool,
    pub interval_secs: u64,
}

impl Default for ProbeOptions {
    fn default() -> Self {
        // 缺省关闭 + 5 分钟。探测会给机场带真实流量，必须显式开启。
        Self {
            enabled: false,
            interval_secs: 300,
        }
    }
}

/// 从 options 行解析探测配置：缺键/解析失败一律落缺省（配置坏了不能炸循环）。
///
/// `rows` 是 `(key, value)`，value 为原始 JSONB（反序列化成 [`serde_json::Value`]）。
pub fn parse_probe_options(rows: &[(String, serde_json::Value)]) -> ProbeOptions {
    let mut opts = ProbeOptions::default();
    for (key, value) in rows {
        match key.as_str() {
            probe_keys::ENABLED => {
                opts.enabled = value.as_bool().unwrap_or(false);
            }
            probe_keys::INTERVAL_SECS => {
                // 下限 60s：误配成 1s 会变成对机场的持续打压。
                opts.interval_secs = value.as_u64().unwrap_or(300).max(60);
            }
            _ => {}
        }
    }
    opts
}

/// 读 options 表 → [`ProbeOptions`]。表不存在/查询失败按缺省（关闭）处理——
/// 探测是可选增强，读取失败不该在日志里刷屏。
async fn read_probe_options(pool: &PgPool) -> ProbeOptions {
    let rows: Vec<(String, serde_json::Value)> =
        sqlx::query_as("SELECT key, value FROM options WHERE key IN ($1, $2)")
            .bind(probe_keys::ENABLED)
            .bind(probe_keys::INTERVAL_SECS)
            .fetch_all(pool)
            .await
            .unwrap_or_default();
    parse_probe_options(&rows)
}

/// 探测目标：第一个 enabled 渠道 base_url 的 host + 443。
///
/// 粗略近似——多渠道走不同上游时只探得到第一个；每渠道独立目标是 M3-C 的事。
/// 无 enabled 渠道或 base_url 解析不出 host 时返回 None（本轮跳过）。
async fn probe_target(pool: &PgPool) -> Option<String> {
    let (base_url,): (String,) = sqlx::query_as(
        "SELECT base_url FROM api_channels WHERE status = 1 AND base_url <> '' ORDER BY priority DESC NULLS LAST LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()?;
    let parsed = url::Url::parse(&base_url).ok()?;
    let host = parsed.host_str()?;
    Some(format!("{host}:443"))
}

/// 启动定时探测循环（生命周期 = 进程生命周期，不做优雅关闭）。
///
/// 每轮都重新读 options——管理台改配置下一轮即生效，不需要 reload。
/// 间隔下限 60s 见 [`parse_probe_options`]。
fn spawn_probe_loop(pool: PgPool, proxies: Arc<gateway_proxy::ProxyManager>) {
    tokio::spawn(async move {
        loop {
            let opts = read_probe_options(&pool).await;
            if opts.enabled {
                match probe_target(&pool).await {
                    Some(target) => {
                        // ponytail: timeout 固定 5s；要可配再加 options 项。
                        let results = proxies
                            .probe_all(&target, std::time::Duration::from_secs(5))
                            .await;
                        let alive = results.iter().filter(|r| r.is_alive()).count();
                        tracing::info!(
                            target = %target,
                            alive,
                            total = results.len(),
                            "proxy probe round finished"
                        );
                    }
                    None => tracing::debug!("probe enabled but no channel base_url to target"),
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(opts.interval_secs)).await;
        }
    });
}
