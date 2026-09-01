//! 密钥管理端点 DTO。
//! 参考: mock::account::ApiKey 形状 + new-api /api/token/。

use crate::records::TokenRecord;
use serde::{Deserialize, Serialize};

/// GET /api/token/ → data: Vec<TokenDto>
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenDto {
    pub key: String,
    pub name: String,
    /// 明文仅出现在创建响应; 列表里恒为 None (前端显示掩码)。
    pub plain_key: Option<String>,
    /// 掩码显示形如 "sk-ab****ef"。
    pub masked_key: String,
    pub group: Option<String>,
    pub quota: i64,
    pub unlimited_quota: bool,
    pub used_quota: i64,
    pub status: u8,
    pub expires_at: Option<String>,
}

impl From<&TokenRecord> for TokenDto {
    fn from(r: &TokenRecord) -> Self {
        Self {
            key: r.meta.key.clone(),
            name: r.name.clone(),
            plain_key: None,
            masked_key: r.key_preview.clone(),
            group: r.group.clone(),
            quota: r.quota,
            unlimited_quota: r.unlimited_quota,
            used_quota: r.used_quota,
            status: r.status,
            expires_at: r
                .expires_at
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string()),
        }
    }
}

/// POST /api/token/ (创建) — 响应 data 里带一次性的明文 key。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTokenRequest {
    pub name: String,
    /// None = 跟随用户组。
    pub group: Option<String>,
    pub quota: i64,
    pub unlimited_quota: bool,
    pub expires_at: Option<String>,
}
