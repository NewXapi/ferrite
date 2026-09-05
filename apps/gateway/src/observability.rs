//! 可观测性 — stdout 人类可读层 + 滚动 JSON 文件层。零自研。

pub fn init_tracing(log_level: &str) {
    use tracing_subscriber::{Layer as _, layer::SubscriberExt, util::SubscriberInitExt};

    let stdout = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        );

    let (non_blocking, _guard) =
        tracing_appender::non_blocking(tracing_appender::rolling::daily("logs", "gateway"));
    let file = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(non_blocking)
        .with_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        );
    let _ = tracing_subscriber::registry()
        .with(stdout)
        .with(file)
        .try_init();
}
