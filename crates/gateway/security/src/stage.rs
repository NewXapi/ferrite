//! `stage` —— `StreamingInterceptStage`：接入 pipeline
//!
//! 在 `forward` 与 `protocol-bridge` 之间执行。接管 `ctx.upstream`（流式），
//! 逐 chunk 过 AC + CtxTail，命中时 sanitize 替换 + 累计 ctx.streamed。

use super::moderation::Moderation;
use super::wordlist::WordList;
use arc_swap::ArcSwap;
use async_trait::async_trait;
use gateway_pipeline::{RequestCtx, Stage, StageError, StageOutcome};
use std::sync::Arc;

// ponytail: 桩 — handle 实现后读取 words/moderation。
#[allow(dead_code)]
pub struct StreamingInterceptStage {
    words: Arc<ArcSwap<WordList>>,
    moderation: Arc<dyn Moderation>,
}

impl StreamingInterceptStage {
    pub fn new(words: Arc<ArcSwap<WordList>>, moderation: Arc<dyn Moderation>) -> Self {
        Self { words, moderation }
    }
}

#[async_trait]
impl Stage for StreamingInterceptStage {
    fn name(&self) -> &'static str {
        "security"
    }

    async fn handle(&self, _ctx: &mut RequestCtx) -> Result<StageOutcome, StageError> {
        // TODO: 接管 ctx.upstream 流 → AC + CtxTail → sanitize → 必要 moderation::check
        unimplemented!("StreamingInterceptStage::handle")
    }
}

/// 重算脱敏后 token 计数（避免计费偏差）
pub fn recount_tokens_after_sanitize(_ctx: &mut RequestCtx) {
    // TODO: 按 sanitized 文本长度重算
    unimplemented!("recount_tokens_after_sanitize")
}
