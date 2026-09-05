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
use std::sync::Arc;

pub fn build_app() -> axum::Router {
    let health = Arc::new(MemoryHealthTable::new());
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

pub fn load_snapshot() -> Arc<Snapshot> {
    Arc::new(Snapshot {
        units: vec![],
        channels: std::collections::HashMap::new(),
    })
}
