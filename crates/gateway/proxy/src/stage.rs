//! `stage` —— `ProxyStage`：接入 pipeline
//!
//! 在 `dispatch` 之后、`forward` 之前执行。负责按 channel 选代理节点，
//! 把拨号结果（`PreparedConn`）写入 `ctx`，供 `forward` 复用。

use std::sync::Arc;
use async_trait::async_trait;
use gateway_pipeline::{
    Stage, RequestCtx, StageOutcome, StageError,
};
use super::pool::ProxyPool;

/// 已拨号的 TCP 连接 + 远端目标地址
pub struct PreparedConn {
    pub stream: tokio::net::TcpStream,
    pub target: std::net::SocketAddr,
}

pub struct ProxyStage {
    pool: Arc<ProxyPool>,
}

impl ProxyStage {
    pub fn new(pool: Arc<ProxyPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Stage for ProxyStage {
    fn name(&self) -> &'static str { "proxy" }

    async fn handle(&self, ctx: &mut RequestCtx) -> Result<StageOutcome, StageError> {
        let route = ctx.route.as_ref()
            .ok_or_else(|| StageError::Internal(anyhow::anyhow!("dispatch not run")))?;

        // TODO: 按 channel_id 选代理 → dial → 写入 ctx
        unimplemented!("ProxyStage::handle")
    }
}
