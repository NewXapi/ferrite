//! Ferrite 网关数据面 — 进程入口，只做组装和启动。
//!
//! 协议转换在 protocol-bridge；调度在 dispatch；准入在 gate；转发在 forward。
//! 本文件把四段链串成真实 axum 服务，注入真实依赖（非 mock）。

mod config;
mod observability;

use std::process::ExitCode;
use std::sync::Arc;

use dispatch::{Dispatcher, MemoryHealthTable, Snapshot};
use forward::egress::ReqwestEgress;
use gateway_gate::chain::GateChain;
use gateway_gate::graylist::GrayListGate;
use gateway_gate::model::ModelGate;
use gateway_gate::quota::QuotaGate;
use gateway_gate::ratelimit::{RateLimitGate, RateLimiter};
use gateway_gate::state::StateGate;
use gateway_pipeline::pipeline::Pipeline;
use gateway_protocol_bridge::adaptor::AdaptorRegistry;

use crate::config::GatewayConfig;
use crate::observability::init_tracing;

use dispatch::stage::DispatchStage;
use forward::stage::ForwardStage;
use gateway_gate::auth::AuthGate;
use gateway_protocol_bridge::stage::ProtocolBridgeStage;

#[tokio::main]
async fn main() -> ExitCode {
    let config = match GatewayConfig::load(std::path::Path::new("config/config.toml")) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to load config: {e}");
            return ExitCode::FAILURE;
        }
    };

    init_tracing(&config.log_level);

    let health = Arc::new(MemoryHealthTable::new());
    let adaptors = Arc::new(AdaptorRegistry::with_defaults());
    let egress = Arc::new(ReqwestEgress::new());

    let snapshot: Arc<Snapshot> = Arc::new(Snapshot {
        units: vec![],
        channels: std::collections::HashMap::new(),
    });
    let dispatcher = Arc::new(Dispatcher::new(Some(snapshot), health.clone()));

    let gates = GateChain::new()
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
        )));

    let pipeline = Arc::new(
        Pipeline::new()
            .push(gates)
            .push(DispatchStage::new(dispatcher))
            .push(ForwardStage::new(egress, adaptors.clone()))
            .push(ProtocolBridgeStage::new(adaptors)),
    );

    let app = gateway_pipeline::router::build_router(pipeline)
        .layer(tower_http::cors::CorsLayer::permissive());

    let listener = match tokio::net::TcpListener::bind(&config.listen).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(error = %e, "failed to bind {}", config.listen);
            return ExitCode::FAILURE;
        }
    };

    tracing::info!(listen = %config.listen, "gateway starting");

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        tracing::error!(error = %e, "server error");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install ctrl-c handler");
    };
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install sigterm handler")
            .recv()
            .await;
    };
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
