mod config;
mod observability;

use crate::config::GatewayConfig;
use crate::observability::init_tracing;
use gateway::build_app;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let mut config = GatewayConfig::load(std::path::Path::new("config/config.toml"))
        .unwrap_or_else(|e| {
            eprintln!("failed to load config: {e}");
            std::process::exit(1);
        });
    init_tracing(&config.log_level);

    let (shutdown_tx, _) = tokio::sync::watch::channel(false);
    let mut _server: Option<tokio::task::JoinHandle<()>> = None;
    let addr = config.listen.clone();

    let tx = shutdown_tx.clone();
    _server = Some(tokio::spawn(async move { serve(&addr, tx).await }));

    let mut hup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
        .expect("install sighup handler");
    loop {
        tokio::select! {
            _ = hup.recv() => {
                tracing::info!("SIGHUP received, reloading config");
                match GatewayConfig::load(std::path::Path::new("config/config.toml")) {
                    Ok(c) => {
                        config = c;
                        init_tracing(&config.log_level);
                        if let Some(h) = _server.take() {
                            let _ = shutdown_tx.send(true);
                            let _ = h.await;
                        }
                        let tx = shutdown_tx.clone();
                        let addr = config.listen.clone();
                        _server = Some(tokio::spawn(async move { serve(&addr, tx).await }));
                        tracing::info!("reload complete, listen={}", config.listen);
                    }
                    Err(e) => tracing::error!(error = %e, "reload config failed"),
                }
            }
        }
    }
}

async fn serve(addr: &str, shutdown: tokio::sync::watch::Sender<bool>) {
    let app = build_app();
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(error = %e, "failed to bind {addr}");
            return;
        }
    };
    tracing::info!(listen = %addr, "gateway serving");
    tokio::select! {
        res = axum::serve(listener, app) => {
            if let Err(e) = res { tracing::error!(error = %e, "serve error"); }
            let _ = shutdown.send(true);
        }
        _ = shutdown_signal() => { let _ = shutdown.send(true); }
    }
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
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("shutdown signal received");
}
