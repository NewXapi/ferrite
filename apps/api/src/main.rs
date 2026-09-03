//! Ferrite — 进程入口，只做组装和启动

use api::config;
use api::dispatch;
use api::gateway;
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

    let pool = match config::init_pool(&config.database_url).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("failed to connect to database: {e}");
            return ExitCode::FAILURE;
        }
    };

    // 统一遥测（纯现成 crate，零自研）：
    //   stdout 人类可读层 + 滚动 JSON 文件层（tracing-appender，非阻塞）
    //   两层各自挂 per-layer EnvFilter，RUST_LOG / config.log_level 统一控制
    use tracing_subscriber::{Layer as _, layer::SubscriberExt, util::SubscriberInitExt};
    let file_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level));
    let stdout_filter = file_filter.clone();
    let (file_writer, log_guard) = tracing_appender::non_blocking(
        tracing_appender::rolling::daily(gateway::LOG_DIR, "ferrite.log"),
    );
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(stdout_filter))
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(file_writer)
                .with_filter(file_filter),
        )
        .init();
    // guard 存活到 main 结束；drop 时阻塞排空非阻塞写线程，所有退出路径都不丢日志
    let _log_guard = log_guard;

    // 桥接：依赖 crate（reqwest/hyper 等）经 log 门面输出的日志统一进 tracing subscriber
    let _ = tracing_log::LogTracer::init();

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
    let tavern = match api::tavern::router(&api::tavern::TavernConfig::default()) {
        Ok(router) => router,
        Err(e) => {
            tracing::error!(error = %e, "failed to initialize tavern storage");
            return ExitCode::FAILURE;
        }
    };
    let app = gateway.router().merge(tavern);

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
