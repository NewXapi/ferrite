//! `auth` —— gate 1：Token 提取 + 哈希查表 + auth_version 校验

use std::sync::Arc;

use arc_swap::ArcSwap;
use async_trait::async_trait;
use sha2::{Digest, Sha256};

use super::TokenInfo;
use super::chain::{Gate, GateCtx};
use super::error::Rejection;
use super::snapshot::TokenSnapshot;
use super::snapshot::TokenView;

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
    fn name(&self) -> &'static str {
        "auth"
    }

    async fn check(&self, ctx: &mut GateCtx) -> Result<(), Rejection> {
        // 1. 提取 key
        let raw = extract_key(&ctx.request_meta)?;
        ctx.raw_key = Some(raw.clone());

        // 2. sha256 哈希查表
        let hash = sha256(&raw);
        let entry = self
            .tokens
            .load()
            .lookup(&hash)
            .ok_or(Rejection::InvalidApiKey)?;
        let token_record = entry.record;

        // 3. 写入 user_key（state gate 用）+ token 信息
        ctx.user_key = Some(token_record.user_key().to_string());
        ctx.token = Some(TokenInfo {
            id: id_from_meta(&token_record.meta.key),
            user_id: 0, // 真实值由 state gate 用 user_key 查回再补
            id_hash: hash,
            group: token_record.group().unwrap_or("").to_string(),
            enabled: token_record.is_enabled(),
            expires_at: token_record.expires_at_unix(),
            allowed_models: entry.allowed_models,
            auth_version: token_record.auth_version(),
        });
        Ok(())
    }
}

/// 从三种标准头里提取 key：Authorization: Bearer / x-api-key / x-goog-api-key
pub fn extract_key(meta: &gateway_pipeline::RequestMeta) -> Result<String, Rejection> {
    if let Some(rest) = meta
        .headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
    {
        return Ok(rest.to_string());
    }
    if let Some(s) = meta.headers.get("x-api-key").and_then(|h| h.to_str().ok()) {
        return Ok(s.to_string());
    }
    if let Some(s) = meta
        .headers
        .get("x-goog-api-key")
        .and_then(|h| h.to_str().ok())
    {
        return Ok(s.to_string());
    }
    Err(Rejection::InvalidApiKey)
}

/// SHA-256 → 32 字节。ponytail: 一次性 `Sha256::new()` + `update` + `finalize`，无堆分配。
pub fn sha256(input: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let out = hasher.finalize();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&out);
    arr
}

/// `meta.key` 字符串约定的数字解析；解析失败 → 0（保守；sync 层负责给合法 id）。
fn id_from_meta(key: &str) -> i64 {
    key.parse().unwrap_or(0)
}
