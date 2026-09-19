//! `stage` —— ProtocolBridgeStage 实现
//!
//! pipeline 的 Stage 4：把 forward 已经准备好的上游响应**打包**成 HTTP 响应，
//! 或在错误路径上按入站协议形状回错误体。
//!
//! ## 分工（WP6 定稿）
//!
//! 协议转换（请求方向 encode + 响应方向 decode）**全部在 forward 内闭环**——
//! 请求方向与响应方向必须协作同一份 codec 实例，而请求方向发生在 forward 内部
//! （那里才同时拿得到请求体与上游格式），把响应转换也放在同一处，方向信息天然
//! 一致，不必跨 stage 传递「这次用的是哪个 encoder」。
//!
//! 旧实现在本 stage 做非流式转换，并把响应源协议**硬编码成 OpenAI**：流式路径
//! 根本不经过本 stage（`ctx.upstream` 只在非流式分支写入），于是流式的
//! Claude/Gemini 客户端永远只拿到 OpenAI 事件形状。这正是 G2。
//!
//! 因此本 stage 现在只做两件事：
//!
//! 1. **错误映射**：`ctx.error` 已被前面 stage 写入时按入站协议回错误体；
//! 2. **打包**：把 `ctx.upstream`（forward 写入，body 已是入站格式）转成 HTTP 响应。

use crate::format_codec::FormatRegistry;
use async_trait::async_trait;
use gateway_pipeline::{RequestCtx, Stage, StageError, StageOutcome};
use std::sync::Arc;

/// 数据面协议出口 stage。
pub struct ProtocolBridgeStage {
    formats: Arc<FormatRegistry>,
}

impl ProtocolBridgeStage {
    /// 用单格式注册表构造（生产装配走 [`FormatRegistry::with_defaults`]）。
    pub fn new(formats: Arc<FormatRegistry>) -> Self {
        Self { formats }
    }

    /// 注册表引用（forward 侧的转换与此共享同一份 codec 实例）。
    pub fn formats(&self) -> &Arc<FormatRegistry> {
        &self.formats
    }
}

#[async_trait]
impl Stage for ProtocolBridgeStage {
    fn name(&self) -> &'static str {
        "protocol-bridge"
    }

    async fn handle(&self, ctx: &mut RequestCtx) -> Result<StageOutcome, StageError> {
        let inbound = ctx.request.inbound_protocol;

        // 错误优先：ctx.error 已被前面 stage 写入则按入站协议形状转换。
        if let Some(e) = ctx.error.take() {
            let resp = crate::error_mapping::map_error(e, inbound);
            return Ok(StageOutcome::ShortCircuit(resp));
        }

        // 正常路径：forward 已把上游响应转成入站格式并写入 ctx.upstream，此处只打包。
        let upstream = ctx.upstream.take().ok_or_else(|| {
            StageError::Internal(anyhow::anyhow!(
                "protocol-bridge: no upstream response and no error to map"
            ))
        })?;

        let resp = http::Response::builder()
            .status(upstream.status)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(axum::body::Body::from(upstream.body))
            .map_err(|e| StageError::Internal(anyhow::anyhow!("build response: {e}")))?;

        Ok(StageOutcome::ShortCircuit(resp))
    }
}
