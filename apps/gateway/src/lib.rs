//! 网关数据面组装 —— 只把各 crate 拼成 pipeline，不做协议/调度/准入判断。

pub mod config;
pub mod observability;

use crate::config::GatewayConfig;
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
use metering::pricing::{ConfigPriceTable, PriceTable};
use std::sync::Arc;

/// 按配置组装 axum router。
///
/// `cfg.dispatch` 决定健康表的冷却参数；`cfg.metering.prices` 非空时装配价格表
/// （空 = 本地单机不计费）；`cfg.retry.max_attempts` 是转发的尝试预算。
pub fn build_app(cfg: &GatewayConfig) -> axum::Router {
    let health = Arc::new(MemoryHealthTable::with_config(HealthSetting {
        cooldown_threshold: cfg.dispatch.cooldown_threshold,
        cooldown_base_seconds: cfg.dispatch.cooldown_base_seconds,
        cooldown_max_seconds: cfg.dispatch.cooldown_max_seconds,
        ..HealthSetting::default()
    }));
    let adaptors = Arc::new(AdaptorRegistry::with_defaults());
    let egress = Arc::new(ReqwestEgress::new());
    let snapshot: Arc<Snapshot> = load_snapshot();
    let dispatcher = Arc::new(Dispatcher::new(Some(snapshot), health.clone()));
    let gates = build_gates();
    let pipeline = Arc::new(
        Pipeline::new()
            .push(gates)
            .push(DispatchStage::new(dispatcher))
            .push(ForwardStage::new(egress, adaptors.clone()))
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

pub fn build_gates() -> GateChain {
    GateChain::new()
        .push(AuthGate::new(Arc::new(arc_swap::ArcSwap::from_pointee(
            gateway_gate::snapshot::TokenSnapshot::default(),
        ))))
        .push(StateGate::new(
            Arc::new(arc_swap::ArcSwap::from_pointee(
                gateway_gate::snapshot::UserSnapshot::default(),
            )),
            Arc::new(arc_swap::ArcSwap::from_pointee(
                gateway_gate::snapshot::IpPolicy::default(),
            )),
        ))
        .push(ModelGate)
        .push(QuotaGate::new(
            Arc::new(arc_swap::ArcSwap::from_pointee(
                gateway_gate::snapshot::QuotaSnapshot::default(),
            )),
            Arc::new(arc_swap::ArcSwap::from_pointee(
                gateway_gate::snapshot::PricingSnapshot::default(),
            )),
        ))
        .push(RateLimitGate::new(Arc::new(RateLimiter::new(100, 60))))
        .push(GrayListGate::new(Arc::new(
            arc_swap::ArcSwap::from_pointee(gateway_gate::graylist::GrayListState::default()),
        )))
}

/// 路由快照占位：真实数据由 admin-sync 推送，进程启动时为空。
pub fn load_snapshot() -> Arc<Snapshot> {
    Arc::new(Snapshot {
        units: vec![],
        channels: std::collections::HashMap::new(),
    })
}
