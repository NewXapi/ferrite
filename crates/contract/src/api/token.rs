//! 密钥管理端点 DTO。
//! 形状对齐后端 `admin-catalog::tokens` 的 `TokenView` / `CreateTokenResult`
//! (camelCase 序列化,2026-09-09 curl 实测)。

use crate::records::TokenRecord;
use serde::{Deserialize, Serialize};

/// GET /api/token → `{items: [TokenDto], total}`;字段对齐 TokenView。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenDto {
    pub key: String,
    pub name: String,
    /// 掩码预览,形如 "sk-ab****ef";后端字段名是 keyPreview。
    #[serde(alias = "keyPreview")]
    pub masked_key: String,
    pub group: Option<String>,
    pub quota: i64,
    pub unlimited_quota: bool,
    pub used_quota: i64,
    pub status: i16,
    pub expires_at: Option<String>,
}

impl From<&TokenRecord> for TokenDto {
    fn from(r: &TokenRecord) -> Self {
        Self {
            key: r.meta.key.clone(),
            name: r.name.clone(),
            masked_key: r.key_preview.clone(),
            group: r.group.clone(),
            quota: r.quota,
            unlimited_quota: r.unlimited_quota,
            used_quota: r.used_quota,
            status: r.status as i16,
            expires_at: r.expires_at.map(|t| t.format("%Y-%m-%d %H:%M").to_string()),
        }
    }
}

/// POST /api/token (创建) 的请求体。
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

/// POST /api/token (创建) 的响应:明文 key 只在这一次出现,
/// 后端形状为 `{plaintext, token: TokenView}` (CreateTokenResult)。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTokenResult {
    pub plaintext: String,
    pub token: TokenDto,
}

/// PUT /api/token/{key} (编辑);后端逐字段 Option,None = 不改。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTokenRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub quota: Option<i64>,
    #[serde(default)]
    pub unlimited_quota: Option<bool>,
    #[serde(default)]
    pub status: Option<i16>,
    #[serde(default)]
    pub expires_at: Option<String>,
}
