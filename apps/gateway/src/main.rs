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

    // 每个 serve 任务配一个独立停止通道：复用同一通道时旧任务的 true 会让
    // 新任务一启动就退出。
    let mut server: Option<(
        tokio::task::JoinHandle<()>,
        tokio::sync::watch::Sender<bool>,
    )> = Some(spawn_server(&config));

    // 信号 handler 在循环外建一次；放进 select! 每轮都会重新注册。
    let mut hup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
        .expect("install sighup handler");
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("install sigterm handler");
    loop {
        tokio::select! {
            _ = hup.recv() => {
                tracing::info!("SIGHUP received, reloading config");
                match GatewayConfig::load(std::path::Path::new("config/config.toml")) {
                    Ok(c) => {
                        config = c;
                        init_tracing(&config.log_level);
                        // 必须等旧任务真正退出：它还占着监听端口，新任务会 bind 失败。
                        stop_server(server.take()).await;
                        server = Some(spawn_server(&config));
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
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("ctrl-c received, shutting down");
                stop_server(server.take()).await;
                return ExitCode::SUCCESS;
            }
            _ = term.recv() => {
                tracing::info!("SIGTERM received, shutting down");
                stop_server(server.take()).await;
                return ExitCode::SUCCESS;
            }
        }
    }
}

/// 起一个 serve 任务，返回句柄与它的停止开关。
fn spawn_server(
    config: &GatewayConfig,
) -> (
    tokio::task::JoinHandle<()>,
    tokio::sync::watch::Sender<bool>,
) {
    let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
    let cfg = Arc::new(config.clone());
    (tokio::spawn(serve(cfg, stop_rx)), stop_tx)
}

async fn serve(cfg: Arc<GatewayConfig>, mut stop: tokio::sync::watch::Receiver<bool>) {
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
    // with_graceful_shutdown 才会让 axum 停止接受新连接并释放监听 socket；
    // 用 select! 包 axum::serve 不行——那个 future 只在自身出错时结束。
    let res = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = stop.wait_for(|v| *v).await;
        })
        .await;
    if let Err(e) = res {
        tracing::error!(error = %e, "serve error");
    }
}

/// 通知 serve 任务停止并等它真正退出（释放监听端口）。
async fn stop_server(
    server: Option<(
        tokio::task::JoinHandle<()>,
        tokio::sync::watch::Sender<bool>,
    )>,
) {
    if let Some((handle, stop)) = server {
        let _ = stop.send(true);
        let _ = handle.await;
    }
}
