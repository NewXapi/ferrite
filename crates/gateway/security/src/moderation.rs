//! `moderation` —— `Moderation` trait + 第三方实现

use async_trait::async_trait;

/// 审核结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModerationResult {
    Pass,
    Block { reason: String, category: String },
}

/// 第三方审核 trait
#[async_trait]
pub trait Moderation: Send + Sync {
    async fn check(&self, text: &str) -> ModerationResult;
}

/// 默认空实现（直接放行）
pub struct Disabled;

#[async_trait]
impl Moderation for Disabled {
    async fn check(&self, _text: &str) -> ModerationResult {
        ModerationResult::Pass
    }
}

/// OpenAI omni-moderation 实现
pub struct OpenAiOmnimod {
    pub api_key: String,
}

#[async_trait]
impl Moderation for OpenAiOmnimod {
    async fn check(&self, _text: &str) -> ModerationResult {
        // TODO: POST /v1/moderations
        unimplemented!("OpenAiOmnimod::check")
    }
}

/// 阿里 Qwen Guard 实现
pub struct QwenGuard {
    pub api_key: String,
}

#[async_trait]
impl Moderation for QwenGuard {
    async fn check(&self, _text: &str) -> ModerationResult {
        // TODO: Qwen Guard API
        unimplemented!("QwenGuard::check")
    }
}
