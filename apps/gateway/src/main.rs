use gateway::build_app;
use gateway::config::GatewayConfig;
use gateway::observability::init_tracing;
use std::process::ExitCode;
use std::sync::Arc;

#[tokio::main]
async fn main() -> ExitCode {
    let mut config = GatewayConfig::load(std::path::Path::new("config/config.toml"))
        .unwrap_or_else(|e| {
            eprintln!("failed to load config: {e}");
            std::process::exit(1);
        });
    init_tracing(&config.log_level);

    let (shutdown_tx, _) = tokio::sync::watch::channel(false);

    let tx = shutdown_tx.clone();
    let cfg = Arc::new(config.clone());
    let mut server = Some(tokio::spawn(async move { serve(cfg, tx).await }));

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
                        if let Some(h) = server.take() {
                            let _ = shutdown_tx.send(true);
                            let _ = h.await;
                        }
                        let tx = shutdown_tx.clone();
                        let cfg = Arc::new(config.clone());
                        server = Some(tokio::spawn(async move { serve(cfg, tx).await }));
                        tracing::info!(
                            listen = %config.listen,
                            cooldown_threshold = config.dispatch.cooldown_threshold,
                            priced_models = config.metering.prices.len(),
                            max_attempts = config.retry.max_attempts,
                            "reload complete",
                        );
                    }
                    Err(e) => tracing::error!(error = %e, "reload config failed"),
                }
            }
        }
    }
}

async fn serve(cfg: Arc<GatewayConfig>, shutdown: tokio::sync::watch::Sender<bool>) {
    let app = build_app(&cfg);
    let listener = match tokio::net::TcpListener::bind(&cfg.listen).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(error = %e, listen = %cfg.listen, "failed to bind");
            return;
        }
    };
    tracing::info!(
        listen = %cfg.listen,
        cooldown_threshold = cfg.dispatch.cooldown_threshold,
        priced_models = cfg.metering.prices.len(),
        max_attempts = cfg.retry.max_attempts,
        "gateway serving",
    );
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
