//! shoes sidecar 生命周期。
//!
//! ferrite 不实现 vless/vmess/ss/trojan/reality 握手；这些协议由独立 shoes
//! 进程在 mixed HTTP+SOCKS5 入站上消化。本模块只负责 spawn / 等端口 / kill。
//! `binary` 为空则不起进程（本地单机默认直连）。

use crate::config::EgressConfig;
use std::net::SocketAddr;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::{Child, Command};

/// 已启动的 shoes 子进程。`stop` 发送 SIGKILL 并 `wait`，避免僵尸。
#[derive(Debug)]
pub struct ShoesSidecar {
    child: Child,
    listen: String,
}

impl ShoesSidecar {
    /// `binary` 为空返回 `Ok(None)`。配置文件缺失、spawn 失败或入站端口超时返回 `Err`。
    pub async fn spawn(cfg: &EgressConfig) -> anyhow::Result<Option<Self>> {
        if cfg.binary.is_empty() {
            return Ok(None);
        }
        let config_path = std::path::Path::new(&cfg.config);
        if !config_path.is_file() {
            anyhow::bail!("shoes config not found: {}", cfg.config);
        }
        let mut child = Command::new(&cfg.binary)
            .arg(&cfg.config)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| anyhow::anyhow!("spawn shoes `{}`: {e}", cfg.binary))?;

        match wait_for_listen(&mut child, &cfg.listen, Duration::from_secs(10)).await {
            Ok(()) => {
                tracing::info!(
                    binary = %cfg.binary,
                    listen = %cfg.listen,
                    config = %cfg.config,
                    "shoes sidecar ready"
                );
                Ok(Some(Self {
                    child,
                    listen: cfg.listen.clone(),
                }))
            }
            Err(e) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                Err(e)
            }
        }
    }

    /// 杀掉子进程并回收。shoes 无 clash-api，热更新靠它自己的文件监视；
    /// 网关 SIGHUP 时整进程重启 sidecar，避免入站地址变更后旧进程占端口。
    pub async fn stop(mut self) {
        tracing::info!(listen = %self.listen, "stopping shoes sidecar");
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }
}

/// 轮询入站端口直到可连；子进程提前退出立刻失败，不用干等到超时。
async fn wait_for_listen(child: &mut Child, listen: &str, timeout: Duration) -> anyhow::Result<()> {
    let addr: SocketAddr = listen
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid [egress].listen `{listen}`: {e}"))?;
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => anyhow::bail!("shoes exited before inbound ready: {status}"),
            Ok(None) => {}
            Err(e) => anyhow::bail!("poll shoes child: {e}"),
        }
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("shoes mixed inbound `{listen}` not ready within {timeout:?}");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
