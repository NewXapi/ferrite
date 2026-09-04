//! `stage` —— ProtocolBridgeStage 实现
//!
//! 这是 pipeline 中的 Stage 4：把上游响应通过 AdaptorRegistry 转换为客户端协议。

use crate::adaptor::{AdaptorRegistry, Protocol};
use async_trait::async_trait;
use gateway_pipeline::{RequestCtx, Stage, StageError, StageOutcome};
use std::sync::Arc;

/// 数据面协议出口 stage
pub struct ProtocolBridgeStage {
    adaptors: Arc<AdaptorRegistry>,
}

impl ProtocolBridgeStage {
    pub fn new(adaptors: Arc<AdaptorRegistry>) -> Self {
        Self { adaptors }
    }
}

#[async_trait]
impl Stage for ProtocolBridgeStage {
    fn name(&self) -> &'static str {
        "protocol-bridge"
    }

    async fn handle(&self, ctx: &mut RequestCtx) -> Result<StageOutcome, StageError> {
        let target = ctx.request.inbound_protocol;

        // 错误优先：ctx.error 已被前面 stage 写入则转换
        if let Some(e) = ctx.error.take() {
            let resp = crate::error_mapping::map_error(e, target);
            return Ok(StageOutcome::ShortCircuit(resp));
        }

        // 正常路径：转换上游响应
        let upstream = ctx
            .upstream
            .take()
            .ok_or_else(|| StageError::Internal(anyhow::anyhow!("forward stage did not run")))?;

        // 把上游协议(OpenAI 中枢) 转为客户端协议; 上游响应体是 OpenAI 形状
        // (forward 透传), 目标协议由入站决定。
        let source = Protocol::OpenAi;
        let target = match target {
            gateway_pipeline::ctx::ProtocolKind::OpenAI => Protocol::OpenAi,
            gateway_pipeline::ctx::ProtocolKind::OpenAIResp => Protocol::OpenAi,
            gateway_pipeline::ctx::ProtocolKind::Anthropic => Protocol::Claude,
            gateway_pipeline::ctx::ProtocolKind::Gemini => Protocol::Gemini,
        };
        let codec = self.adaptors.resolve(source, target).ok_or_else(|| {
            StageError::Internal(anyhow::anyhow!("no adaptor for {source:?} -> {target:?}"))
        })?;

        // 非流式: 上游响应整体字节 → 目标协议字节 → 客户端响应
        let converted = codec
            .adapt_response(upstream.body)
            .map_err(|e| StageError::Internal(anyhow::anyhow!("adapt response failed: {e}")))?;
        let body = converted.into_iter().flatten().collect::<bytes::Bytes>();
        let resp = http::Response::builder()
            .status(upstream.status)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(axum::body::Body::from(body))
            .map_err(|e| StageError::Internal(anyhow::anyhow!("build response: {e}")))?;

        Ok(StageOutcome::ShortCircuit(resp))
    }
}
