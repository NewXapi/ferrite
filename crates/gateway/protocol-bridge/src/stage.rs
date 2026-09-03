//! `stage` —— ProtocolBridgeStage 实现
//!
//! 这是 pipeline 中的 Stage 4：把上游响应通过 CodecRegistry 转换为客户端协议。

use std::sync::Arc;
use async_trait::async_trait;
use gateway_pipeline::{
    Stage, RequestCtx, StageOutcome, StageError,
};
use crate::codec::CodecRegistry;

/// 数据面协议出口 stage
pub struct ProtocolBridgeStage {
    codecs: Arc<CodecRegistry>,
}

impl ProtocolBridgeStage {
    pub fn new(codecs: Arc<CodecRegistry>) -> Self {
        Self { codecs }
    }
}

#[async_trait]
impl Stage for ProtocolBridgeStage {
    fn name(&self) -> &'static str { "protocol-bridge" }

    async fn handle(&self, ctx: &mut RequestCtx) -> Result<StageOutcome, StageError> {
        let target = ctx.request.inbound_protocol;

        // 错误优先：ctx.error 已被前面 stage 写入则转换
        if let Some(e) = ctx.error.take() {
            let resp = crate::error_mapping::map_error(e, target);
            return Ok(StageOutcome::ShortCircuit(resp));
        }

        // 正常路径：转换上游响应
        let upstream = ctx.upstream.take()
            .ok_or_else(|| StageError::Internal(anyhow::anyhow!("forward stage did not run")))?;

        let codec = self.codecs.get(target)
            .ok_or_else(|| StageError::Internal(anyhow::anyhow!(
                "no codec registered for {:?}", target
            )))?;

        let resp = codec.encode(upstream)
            .map_err(|e| StageError::Internal(anyhow::anyhow!("codec encode failed: {e}")))?;

        Ok(StageOutcome::ShortCircuit(resp))
    }
}
