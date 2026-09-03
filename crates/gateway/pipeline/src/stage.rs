//! `stage` —— Stage trait + 错误 / 返回值定义
//!
//! 所有具体 stage（AdmissionStage / DispatchStage / ForwardStage / ProtocolBridgeStage）
//! 都实现本 trait。

use async_trait::async_trait;
use thiserror::Error;
use crate::ctx::{RequestCtx, StageOutcome, UpstreamResponse};

/// 链路上的一个处理节点
#[async_trait]
pub trait Stage: Send + Sync {
    /// stage 名称，用于 tracing span / metrics 标签
    fn name(&self) -> &'static str;

    /// 处理逻辑。返回 `Err` 即短路。
    async fn handle(&self, ctx: &mut RequestCtx) -> Result<StageOutcome, StageError>;
}

/// 上游调用错误（hyper / 解析 / 业务失败）
#[derive(Debug, Error)]
pub enum UpstreamError {
    #[error("connect timeout")]
    Timeout,
    #[error("connect failed: {0}")]
    Connect(#[from] std::io::Error),
    #[error("upstream status {code}: {body_preview:?}")]
    Status { code: u16, body_preview: Vec<u8> },
    #[error("ssrf detected")]
    SSRF,
    #[error("body read error: {0}")]
    Body(#[from] bytes::TryGetError),
}

/// 跨 stage 的统一错误
#[derive(Debug, Error)]
pub enum StageError {
    #[error("unauthenticated: {0}")]
    Unauthenticated(String),

    #[error("quota exhausted: remaining={remaining} required={required}")]
    QuotaExhausted { remaining: i64, required: i64 },

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("no available route")]
    NoRoute,

    #[error("payload too large")]
    PayloadTooLarge,

    #[error("upstream error: {0}")]
    Upstream(#[from] UpstreamError),

    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}
