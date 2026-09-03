//! `auth` —— gate 1：Token 提取 + 哈希查表 + auth_version 校验

use async_trait::async_trait;
use std::sync::Arc;
use arc_swap::ArcSwap;
use super::chain::{Gate, GateCtx};
use super::error::Rejection;
use super::TokenInfo;
use super::snapshot::TokenSnapshot;

pub struct AuthGate {
    tokens: Arc<ArcSwap<TokenSnapshot>>,
}

impl AuthGate {
    pub fn new(tokens: Arc<ArcSwap<TokenSnapshot>>) -> Self {
        Self { tokens }
    }
}

#[async_trait]
impl Gate for AuthGate {
    fn name(&self) -> &'static str { "auth" }

    async fn check(&self, ctx: &mut GateCtx) -> Result<(), Rejection> {
        // 1. 提取 key
        let raw = extract_key(&ctx.request_meta)?;
        ctx.raw_key = Some(raw.clone());

        // 2. sha256 哈希查表
        let hash = sha256(&raw);
        let token_record = self.tokens.load().lookup(&hash)
            .ok_or(Rejection::InvalidApiKey)?;

        // 3. auth_version 单调性（密码/2FA 变更后旧 token 立即失效）
        // 由 state gate 配合 user.auth_version 检查；这里只填充 token 信息
        ctx.token = Some(TokenInfo {
            id: token_record.id,
            user_id: token_record.user_id,
            id_hash: hash,
            group: token_record.group.unwrap_or_default(),
            enabled: token_record.enabled,
            expires_at: token_record.expires_at,
            allowed_models: token_record.allowed_models,
            auth_version: token_record.auth_version,
        });
        Ok(())
    }
}

fn extract_key(meta: &gateway_pipeline::RequestMeta) -> Result<String, Rejection> {
    if let Some(h) = meta.headers.get("authorization") {
        if let Ok(s) = h.to_str() {
            if let Some(rest) = s.strip_prefix("Bearer ") {
                return Ok(rest.to_string());
            }
        }
    }
    if let Some(h) = meta.headers.get("x-api-key") {
        if let Ok(s) = h.to_str() {
            return Ok(s.to_string());
        }
    }
    if let Some(h) = meta.headers.get("x-goog-api-key") {
        if let Ok(s) = h.to_str() {
            return Ok(s.to_string());
        }
    }
    Err(Rejection::InvalidApiKey)
}

fn sha256(input: &str) -> [u8; 32] {
    // TODO: 实际 SHA-256 实现（用 sha2 crate）
    unimplemented!("sha256")
}
