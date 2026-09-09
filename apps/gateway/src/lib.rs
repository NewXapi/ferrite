//! 网关数据面组装 —— 只把各 crate 拼成 pipeline，不做协议/调度/准入判断。

pub mod config;
pub mod observability;

use crate::config::GatewayConfig;
use crate::config::{build_proxy_snapshot, build_route_snapshot, build_token_snapshot};
use dispatch::health::HealthSetting;
use dispatch::stage::DispatchStage;
use dispatch::{Dispatcher, MemoryHealthTable, Snapshot};
use forward::egress::ReqwestEgress;
use forward::stage::ForwardStage;
use gateway_gate::auth::AuthGate;
use gateway_gate::chain::GateChain;
use gateway_gate::graylist::GrayListGate;
use gateway_gate::model::ModelGate;
use gateway_gate::quota::QuotaGate;
use gateway_gate::ratelimit::{RateLimitGate, RateLimiter};
use gateway_gate::state::StateGate;
use gateway_pipeline::pipeline::Pipeline;
use gateway_protocol_bridge::adaptor::AdaptorRegistry;
use gateway_protocol_bridge::stage::ProtocolBridgeStage;
use gateway_proxy::ProxyManager;
use metering::pricing::{ConfigPriceTable, PriceTable};
use std::sync::Arc;

/// 按配置组装 axum router。
///
/// `cfg.channels` 变成 dispatch 的路由快照，`cfg.keys` 变成 gate 的 token 快照；
/// `cfg.proxy_nodes` 注入 `ProxyManager`（仅 HTTP/SOCKS5）；`cfg.dispatch` 决定健康表
/// 的冷却参数；`cfg.metering.prices` 非空时装配价格表并挂上额度闸（空 = 本地单机
/// 不计费）；`cfg.retry.max_attempts` 是转发的尝试预算。
pub fn build_app(cfg: &GatewayConfig) -> axum::Router {
    let health = Arc::new(MemoryHealthTable::with_config(HealthSetting {
        cooldown_threshold: cfg.dispatch.cooldown_threshold,
        cooldown_base_seconds: cfg.dispatch.cooldown_base_seconds,
        cooldown_max_seconds: cfg.dispatch.cooldown_max_seconds,
        ..HealthSetting::default()
    }));
    let adaptors = Arc::new(AdaptorRegistry::with_defaults());
    let egress = Arc::new(ReqwestEgress::new());
    let proxies = Arc::new(ProxyManager::new());
    proxies.install(build_proxy_snapshot(&cfg.proxy_nodes));
    let snapshot: Arc<Snapshot> = load_snapshot(cfg);
    let dispatcher = Arc::new(Dispatcher::new(Some(snapshot), health.clone()));
    let gates = build_gates(cfg);
    let pipeline = Arc::new(
        Pipeline::new()
            .push(gates)
            .push(DispatchStage::new(dispatcher))
            .push(ForwardStage::new(egress, adaptors.clone()).with_proxies(proxies))
            .push(ProtocolBridgeStage::new(adaptors)),
    );
    gateway_pipeline::router::build_router(pipeline)
        .layer(tower_http::cors::CorsLayer::permissive())
}

/// 价格表：`[metering.prices]` 为空返回 `None`（本地单机不计费，转发不受影响）。
pub fn build_price_table(cfg: &GatewayConfig) -> Option<Arc<dyn PriceTable>> {
    if cfg.metering.prices.is_empty() {
        return None;
    }
    Some(Arc::new(ConfigPriceTable::new(cfg.metering.prices.clone())))
}

/// 转发重试预算。
pub fn build_retry_policy(cfg: &GatewayConfig) -> dispatch::RetryPolicy {
    dispatch::RetryPolicy {
        max_attempts: cfg.retry.max_attempts,
    }
}

/// 组装准入闸链。
///
/// `cfg.keys` 变成 token / user 快照供 `AuthGate` 与 `StateGate` 查表；
/// key 列表为空则任何请求都 401。
///
/// `QuotaGate` 只在 `[metering.prices]` 非空时挂上：`QuotaSnapshot::remaining`
/// 对未登记的 token 返回 0，单机不计费时挂它会让每个请求恒 402。计费模式下
/// 额度快照由计量侧写入，才有真实余额可比。
pub fn build_gates(cfg: &GatewayConfig) -> GateChain {
    let (tokens, users) = build_token_snapshot(&cfg.keys);
    let mut chain = GateChain::new()
        .push(AuthGate::new(Arc::new(arc_swap::ArcSwap::from_pointee(
            tokens,
        ))))
        .push(StateGate::new(
            Arc::new(arc_swap::ArcSwap::from_pointee(users)),
            Arc::new(arc_swap::ArcSwap::from_pointee(
                gateway_gate::snapshot::IpPolicy::default(),
            )),
        ))
        .push(ModelGate);
    if !cfg.metering.prices.is_empty() {
        chain = chain.push(QuotaGate::new(
            Arc::new(arc_swap::ArcSwap::from_pointee(
                gateway_gate::snapshot::QuotaSnapshot::default(),
            )),
            Arc::new(arc_swap::ArcSwap::from_pointee(
                gateway_gate::snapshot::PricingSnapshot::default(),
            )),
        ));
    }
    chain
        .push(RateLimitGate::new(Arc::new(RateLimiter::new(100, 60))))
        .push(GrayListGate::new(Arc::new(
            arc_swap::ArcSwap::from_pointee(gateway_gate::graylist::GrayListState::default()),
        )))
}

/// 路由快照 —— 单机模式下由 `[[channels]]` 直接构造。
///
/// 不依赖 admin-sync：配置就是唯一数据源，SIGHUP 重载配置即换快照
/// （`main.rs` 重建整个 app）。渠道为空时任何转发请求返回 404 no_route。
pub fn load_snapshot(cfg: &GatewayConfig) -> Arc<Snapshot> {
    Arc::new(build_route_snapshot(&cfg.channels))
}
