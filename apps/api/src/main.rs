//! Ferrite — 进程入口，只做组装和启动

mod adapter;
mod config;
mod dispatch;
mod gateway;
mod identity;
mod ratelimit;

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let config = match config::load_config(std::path::Path::new("config/config.toml")) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("failed to load config: {e}");
            return ExitCode::FAILURE;
        }
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level)),
        )
        .init();

    let pool = match config::init_pool(&config.database_url).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("failed to connect to database: {e}");
            return ExitCode::FAILURE;
        }
    };
    tracing::info!("database connected");

    let route_index = dispatch::RouteIndex::new();
    match gateway::load_channels(&pool).await {
        Ok(channels) => {
            tracing::info!("loaded {} channels", channels.len());
            route_index.build_from_channels(&channels);
        }
        Err(e) => {
            tracing::warn!("failed to load channels: {e} (starting with empty route index)");
        }
    }

    let gateway = gateway::Gateway::new(pool, route_index);
    let app = gateway.router();

    let listener = match tokio::net::TcpListener::bind(&config.listen).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("failed to bind {}: {e}", config.listen);
            return ExitCode::FAILURE;
        }
    };

    tracing::info!("listening on {}", config.listen);

    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("shutdown signal received");
    };

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
    {
        tracing::error!("server error: {e}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
