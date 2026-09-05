//! Ferrite — 进程入口，只做组装和启动
//!
//! 组装逻辑在 `api::build_app`（lib crate）中，供 e2e 复用。
//! 本文件只负责：读配置 → 建 pool → 初始化遥测 → 构建 app → serve。

use std::net::SocketAddr;
use std::process::ExitCode;

use api::config;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

use crate::config::Config;

/// 日志文件目录（tracing_appender 滚动写入用）
pub const LOG_DIR: &str = "logs";

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

    // 统一遥测：stdout 人类可读层 + 滚动 JSON 文件层（tracing-appender，非阻塞）
    let file_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level));
    let stdout_filter = file_filter.clone();
    let (file_writer, log_guard) =
        tracing_appender::non_blocking(tracing_appender::rolling::daily(LOG_DIR, "ferrite.log"));
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(stdout_filter))
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(file_writer)
                .with_filter(file_filter),
        )
        .init();
    let _log_guard = log_guard;
    let _ = tracing_log::LogTracer::init();

    tracing::info!("database connected");

    match api::build_app(pool, &config).await {
        Ok(app) => serve(app, &config.listen).await,
        Err(e) => {
            tracing::error!("failed to build app: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn serve(app: axum::Router, listen: &str) -> ExitCode {
    let listener = match tokio::net::TcpListener::bind(listen).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("failed to bind {listen}: {e}");
            return ExitCode::FAILURE;
        }
    };
    tracing::info!(listen = %listen, "ferrite serving");

    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("shutdown signal received");
    };

    if let Err(e) = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown)
        .await
    {
        tracing::error!("server error: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
