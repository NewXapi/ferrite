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

use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use sqlx::PgPool;

use crate::config::Config;

pub mod billing;
pub mod config;
pub mod snapshot;
pub mod tavern;

use dispatch::{Dispatcher, MemoryHealthTable};
use forward::egress::ReqwestEgress;
use forward::stage::ForwardStage;
use gateway_gate::auth::AuthGate;
use gateway_gate::chain::GateChain;
use gateway_gate::graylist::GrayListGate;
use gateway_gate::model::{GroupModelGate, ModelGate};
use gateway_gate::quota::QuotaGate;
use gateway_gate::ratelimit::{RateLimitGate, RateLimiter};
use gateway_gate::snapshot::{IpPolicy, PricingSnapshot};
use gateway_gate::state::StateGate;
use gateway_pipeline::pipeline::Pipeline;
use gateway_protocol_bridge::adaptor::AdaptorRegistry;
use gateway_protocol_bridge::stage::ProtocolBridgeStage;

/// 组装完整应用 Router：admin-api + tavern + pipeline gateway（含计费结算）+ reload。
pub async fn build_app(pool: PgPool, _cfg: &Config) -> anyhow::Result<Router> {
    let egress: Arc<dyn forward::egress::Egress> = Arc::new(ReqwestEgress::new());
    assemble(pool, egress, true).await
}

/// 组装完整应用 Router 的公共实现。
///
/// `egress` 可注入（e2e 传 mock），生产路径由 [`build_app`] 传入 `ReqwestEgress`。
///
/// `wire_proxy_pool`：生产 true — 模型请求经 [`gateway_proxy::ProxyManager`]
/// 租出口 Client（直连/代理）。测试 false — ForwardStage 不接代理池，上游
/// 一律走注入的 `egress`：`build_app_with_egress` 的文档契约是"不发起真实
/// 上游请求"，而代理池对无节点渠道也返回直连 reqwest Client，会绕过 mock
/// 去拨 base_url 真实地址（e2e 里是不可达的假地址 → 502）。
async fn assemble(
    pool: PgPool,
    egress: Arc<dyn forward::egress::Egress>,
    wire_proxy_pool: bool,
) -> anyhow::Result<Router> {
    // 建表必须先于任何查询：admin_router::router 内部跑 db_bootstrap::run_migrations，
    // 而 load_proxy_snapshot 查 proxy_nodes（迁移 0005 才建）。顺序颠倒则空库首启失败。
    let proxies = Arc::new(gateway_proxy::ProxyManager::new());

    // JWT secret 属组装关注点：admin-router 只接收现成的 AuthService，不读环境变量。
    let secret = match std::env::var("FERRITE_JWT_SECRET") {
        Ok(s) => s,
        Err(_) => anyhow::bail!("FERRITE_JWT_SECRET env var required"),
    };
    let auth_svc = Arc::new(auth::AuthService::new(pool.clone(), secret.into_bytes())?);

    // admin-api 聚合路由（内部已含 auth，不再单独挂载 auth::router）；
    // auth_svc 同时留给 reload 路由的 bearer 鉴权（见 ReloadState）。
    let admin = admin_router::router(pool.clone(), auth_svc.clone(), proxies.clone())
        .await
        .map_err(|e| anyhow::anyhow!("failed to initialize admin router: {e}"))?;

    // 出口代理池：DB proxy_nodes 表（enabled）→ ProxyManager；
    // 管理台 CRUD 会原地 reload（见 admin-router /api/proxy_nodes）。
    proxies.install(admin_proxy::load_proxy_snapshot(&pool).await?);
    // 主动探测循环（M3-B）：默认关闭，options 表 proxy.probe_enabled=true 才开。
    // 与上一行同因依赖迁移建的表，必须在 run_migrations 之后。
    spawn_probe_loop(pool.clone(), proxies.clone());

    // 酒馆域路由
    let tavern = tavern::router(&tavern::TavernConfig::default())?;

    // 从 PG 加载快照 → Dispatcher + gates → Pipeline
    // Arc 包装是 reload 的前提：ReloadState 与 gate / 计费组件必须共享同一批
    // Shared* 实例，reload 时 store 新值双方才自动可见。
    let snapshots = Arc::new(snapshot::load_snapshots(&pool).await?);
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
        // 组级白名单紧随 token 级 ModelGate：两道闸门取交集（token 白名单先过，
        // 组白名单再过）。GroupModelGate 依赖 ModelGate 已解析出的 ctx.requested_model，
        // 自身不解析请求体；组未配置 / 白名单空 → fail-open（见 gate crate 文档）。
        .push(GroupModelGate::new(snapshots.group_snapshot.clone()))
        .push(QuotaGate::new(
            snapshots.quota_snapshot.clone(),
            Arc::new(arc_swap::ArcSwap::from_pointee(PricingSnapshot::default())),
        ))
        .push(RateLimitGate::new(Arc::new(RateLimiter::new(100, 60))))
        .push(GrayListGate::new(Arc::new(
            arc_swap::ArcSwap::from_pointee(gateway_gate::graylist::GrayListState::default()),
        )));

    let adaptors = Arc::new(AdaptorRegistry::with_defaults());

    // 出口接线：生产接代理池（租约 Client 优先），测试保持 mock egress 直通。
    let forward_stage = ForwardStage::new(egress, adaptors.clone());
    let forward_stage = if wire_proxy_pool {
        forward_stage.with_proxies(proxies.clone())
    } else {
        forward_stage
    };

    // 计费权威接线（#146/#159）：pipeline 结算点是唯一扣费写点，价格表 +
    // 结算 sink 在此注入 ForwardStage；usage 中间件已退役（双写双扣 + SSE
    // 吞流，裁决见 billing.rs 模块文档）。
    // channel_names 从 boot 渠道快照投影（UUID → 展示名）；与 Snapshots.dispatch
    // 字段同款陈旧性——reload 不回写，渠道改名后新账单仍记旧名（Suspect 同价格表）。
    let channel_names: HashMap<String, String> = snapshots
        .dispatch
        .channels
        .iter()
        .map(|(key, channel)| (key.clone(), channel.name.clone()))
        .collect();
    let price_rows = snapshots.price_rows.load();
    let price_table = billing::PgPriceTable::new(&price_rows, snapshots.group_snapshot.clone());
    let settle_sink = billing::PgSettleSink::new(
        pool.clone(),
        snapshots.quota_snapshot.clone(),
        channel_names,
        snapshots.name_directory.clone(),
    );
    let forward_stage =
        forward_stage.with_price_table(Arc::new(price_table), Arc::new(settle_sink));

    let pipeline = Arc::new(
        Pipeline::new()
            .push(gates)
            // dispatcher 同时被 reload 路由（ReloadState）与重试循环持有，clone 一份给 stage；
            // with_retry 后 ForwardStage 自己驱动选路，不再需要 DispatchStage（避免双次 select/限流计数）
            .push(forward_stage.with_retry(dispatcher.clone(), dispatch::RetryPolicy::default()))
            .push(ProtocolBridgeStage::new(adaptors)),
    );

    // 路径守卫包 pipeline router：pipeline 只接数据面路径
    // （/v1* 与 /healthz），其余未匹配路径一律 404，不让 admin/tavern 之外
    // 的杂路径掉进 pipeline 被 AuthGate 判 401（恢复 scoped fallback 语义）。
    let pipeline_router = gateway_pipeline::router::build_router(pipeline)
        .layer(axum::middleware::from_fn(gateway_path_guard));

    // reload 端点：bearer 鉴权 + admin 守卫 + 热更快照（Shared* store + set_snapshot）
    let reload_state = ReloadState {
        pool: pool.clone(),
        dispatcher: dispatcher.clone(),
        snapshots: snapshots.clone(),
        auth_svc,
    };
    let reload = Router::new().route(
        "/api/gateway/reload",
        axum::routing::post(reload_handler).with_state(reload_state),
    );

    // 合并：具体路由优先，pipeline 作为 fallback 兜底 /v1/*
    // UI JSON(/api, /tavern) 统一 no-store：管理台数据不缓存，防止浏览器把
    // 旧响应（含 dev 代理误配期的错误页）持久化重放。/v1 数据面透传上游语义，不加。
    Ok(admin
        .merge(tavern)
        .merge(reload)
        .merge(pipeline_router)
        .layer(axum::middleware::from_fn(no_store_ui_json)))
}

/// /api、/tavern 前缀响应附加 `Cache-Control: no-store`。
async fn no_store_ui_json(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let scoped = req.uri().path().starts_with("/api") || req.uri().path().starts_with("/tavern");
    let mut resp = next.run(req).await;
    if scoped {
        resp.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("no-store"),
        );
    }
    resp
}

/// 测试用：注入 mock egress 构建 Router（不发起真实上游请求）。
pub async fn build_app_with_egress(
    pool: PgPool,
    egress: std::sync::Arc<dyn forward::egress::Egress>,
) -> anyhow::Result<Router> {
    assemble(pool, egress, false).await
}

/// pipeline 数据面路径守卫：只放行 `/healthz`、`/v1`、`/v1/*`、`/v1beta*`。
///
/// 合并后的 Router 里 pipeline 提供 fallback；不守卫则任意未匹配路径
/// （如拼错的 /api/*）会掉进 pipeline 被 AuthGate 判 401，把"路径不存在"
/// 误报成"未认证"。守卫返回纯文本 404（对齐 e2e 契约 `not found`）。
async fn gateway_path_guard(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let path = req.uri().path();
    let scoped = path == "/healthz"
        || path == "/v1"
        || path.starts_with("/v1/")
        || path.starts_with("/v1beta");
    if scoped {
        next.run(req).await
    } else {
        (axum::http::StatusCode::NOT_FOUND, "not found").into_response()
    }
}

/// POST /api/gateway/reload 的路由 state：热更目标 + 鉴权服务。
///
/// `snapshots` 必须与 gate / 计费 sink 持有的是同一个 `Arc<Snapshots>`
/// （boot 时 clone 同源），否则 store 新值后持有者看不到。
/// `dispatcher` 同理：与 DispatchStage 共享同一 `Arc<Dispatcher>`。
/// axum state 要求 Clone，全部字段都是廉价 Arc/pool 句柄克隆。
#[derive(Clone)]
struct ReloadState {
    pool: PgPool,
    dispatcher: Arc<Dispatcher>,
    snapshots: Arc<snapshot::Snapshots>,
    auth_svc: Arc<auth::AuthService>,
}

/// reload 失败响应形状（对齐 admin-catalog 的 err_json 模式）。
type ReloadErrResp = (StatusCode, Json<serde_json::Value>);

/// POST /api/gateway/reload — 热重载管理面快照（channels / route_units / tokens / users / groups）。
///
/// 鉴权与守卫先于任何查库/热更：
/// - 无 Authorization 头或 token 无效/过期 → 401（`bearer_user` 的 `AuthError` 状态码映射）
/// - role < [`auth::routes::ADMIN_ROLE_THRESHOLD`] → 403（对齐 admin-catalog `require_admin`）
///
/// 成功 → 200 `{"success": true, "data": {channels, route_units, tokens, users, groups}}`；
/// 加载/store 失败（DB 错误等）→ 500 `{"success": false, "message": ...}`，运行时
/// 快照保持原样（store 只在加载全部成功后发生）。
async fn reload_handler(
    State(state): State<ReloadState>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<serde_json::Value>), ReloadErrResp> {
    // 1. bearer 鉴权：无头/坏 token → 401（不查管理表，先拒绝）
    let user = auth::routes::bearer_user(&state.auth_svc, &headers)
        .await
        .map_err(|e| {
            (
                e.status(),
                Json(serde_json::json!({ "code": e.code(), "message": e.to_string() })),
            )
        })?;

    // 2. admin 守卫：role >= 10（10=admin, 100=root）
    if user.role < auth::routes::ADMIN_ROLE_THRESHOLD {
        return Err((
            StatusCode::FORBIDDEN,
            Json(
                serde_json::json!({ "code": "FORBIDDEN", "message": "forbidden: admin required" }),
            ),
        ));
    }

    // 3. 热更：加载最新管理表数据 → store 进 Shared* 与 Dispatcher（非原子五次
    // store，见 snapshot::reload_snapshots 文档）
    let counts = snapshot::reload_snapshots(&state.pool, &state.snapshots, &state.dispatcher)
        .await
        .map_err(|e| {
            tracing::error!("gateway snapshot reload failed: {e:#}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "success": false, "message": e.to_string() })),
            )
        })?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "success": true, "data": counts })),
    ))
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
/// 探测是可选增强，读失败不该刷屏，但也不能零可观测：debug 级每间隔最多一条。
async fn read_probe_options(pool: &PgPool) -> ProbeOptions {
    let rows: Vec<(String, serde_json::Value)> =
        match sqlx::query_as("SELECT key, value FROM options WHERE key IN ($1, $2)")
            .bind(probe_keys::ENABLED)
            .bind(probe_keys::INTERVAL_SECS)
            .fetch_all(pool)
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::debug!(error = %e, "读取探测 options 失败，本轮按缺省（关闭）");
                Vec::new()
            }
        };
    parse_probe_options(&rows)
}

/// 每渠道探测目标：输入 enabled 渠道的 `(name, base_url)` 行，输出
/// `(渠道名, "host:443")`。
///
/// base_url 解析失败 / host 为空的行**跳过**（该行没有可用目标）。端口近似
/// 维持 M3-B 现状：无显式端口一律 443；显式非 443 端口的行跳过——探测恒拨
/// 443，给 8443 渠道探 443 是探错目标，跳过好于误报。
pub fn channel_probe_targets(rows: &[(String, String)]) -> Vec<(String, String)> {
    rows.iter()
        .filter_map(|(name, base_url)| {
            let parsed = url::Url::parse(base_url).ok()?;
            let host = parsed.host_str().filter(|h| !h.is_empty())?;
            if parsed.port().is_some_and(|p| p != 443) {
                return None;
            }
            Some((name.clone(), format!("{host}:443")))
        })
        .collect()
}

/// 读全部 enabled 渠道的 `(name, base_url)` 行（供 [`channel_probe_targets`]）。
/// 查询失败按空处理（与 [`read_probe_options`] 同款 debug 日志，不刷屏但可观测）。
async fn probe_targets_rows(pool: &PgPool) -> Vec<(String, String)> {
    match sqlx::query_as(
        "SELECT name, base_url FROM api_channels WHERE status = 1 AND base_url <> '' ORDER BY priority DESC NULLS LAST",
    )
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::debug!(error = %e, "读取探测目标渠道失败，本轮跳过");
            Vec::new()
        }
    }
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
                // 每渠道独立目标（M3-C）：串行遍历渠道，渠道内并发由
                // probe_channel 的分块管。ponytail: 渠道数十级，串行足够；
                // 渠道多了再考虑跨渠道重叠。
                for (channel, target) in channel_probe_targets(&probe_targets_rows(&pool).await) {
                    // ponytail: timeout 固定 5s；要可配再加 options 项。
                    let results = proxies
                        .probe_channel(&channel, &target, std::time::Duration::from_secs(5))
                        .await;
                    let alive = results.iter().filter(|r| r.is_alive()).count();
                    tracing::info!(
                        channel = %channel,
                        target = %target,
                        alive,
                        total = results.len(),
                        "proxy probe round finished"
                    );
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(opts.interval_secs)).await;
        }
    });
}
