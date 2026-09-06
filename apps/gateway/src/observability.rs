//! 可观测性 — stdout 人类可读层 + 滚动 JSON 文件层。零自研。

use std::sync::OnceLock;
use tracing_appender::non_blocking::WorkerGuard;

/// `tracing_appender::non_blocking` 的 guard 一旦 drop，后台写线程就关闭、
/// 文件日志静默失效。存进 `OnceLock` 让它活到进程结束。
static APPENDER_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

/// 初始化 tracing。
///
/// 只有首次调用会装 subscriber（`tracing` 的全局 subscriber 不可替换）；
/// SIGHUP reload 时重复调用是空操作，`log_level` 变更需重启进程才生效。
pub fn init_tracing(log_level: &str) {
    use tracing_subscriber::{Layer as _, layer::SubscriberExt, util::SubscriberInitExt};

    let stdout = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        );

    let (non_blocking, guard) =
        tracing_appender::non_blocking(tracing_appender::rolling::daily("logs", "gateway"));
    let file = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(non_blocking)
        .with_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        );
    if tracing_subscriber::registry()
        .with(stdout)
        .with(file)
        .try_init()
        .is_ok()
    {
        let _ = APPENDER_GUARD.set(guard);
    }
}
