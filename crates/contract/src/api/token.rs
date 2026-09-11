//! 密钥管理端点 DTO。
//! 与后端 `admin-catalog` tokens 的 wire 形状对齐（camelCase，列表为 `{items}` 信封）。

use crate::records::TokenRecord;
use serde::{Deserialize, Serialize};

/// GET /api/token → data: TokenList（后端列表响应为 `{ "items": [...] }` 信封）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenList {
    pub items: Vec<TokenDto>,
}

/// 单个 API 密钥视图（对应后端 TokenView wire，camelCase）。
///
/// `plain_key` 仅出现在创建响应 [`CreateTokenResult`]；列表里恒为 None（前端显示掩码）。
/// `status`: 1=启用 2=停用（对齐 new-api）。
/// `expires_at` / `created_at`: RFC3339 字符串；`expires_at` 为 None 表示永不过期。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenDto {
    /// 密钥 UUID（update/delete 的路径参数）。
    pub key: String,
    /// 所属用户 UUID（owner 模式下等于当前用户；admin_all 列表里可能不同）。
    #[serde(default)]
    pub user_key: String,
    pub name: String,
    /// 明文仅出现在创建响应；列表里恒为 None（前端显示掩码）。
    #[serde(default)]
    pub plain_key: Option<String>,
    /// 后端返回的掩码预览（形如 `sk-ab****ef`），列表展示用它，不再前端自拼。
    #[serde(default)]
    pub key_preview: String,
    /// 分组别名；None = 跟随用户组。
    pub group: Option<String>,
    /// 独立限额（额度单位，500_000≈$1）；`unlimited_quota` 为 true 时无意义。
    pub quota: i64,
    pub unlimited_quota: bool,
    pub used_quota: i64,
    /// 1 启用 / 2 停用。
    pub status: u8,
    /// RFC3339；None = 永不过期。
    pub expires_at: Option<String>,
    /// RFC3339 创建时间。
    #[serde(default)]
    pub created_at: String,
}

impl From<&TokenRecord> for TokenDto {
    fn from(r: &TokenRecord) -> Self {
        Self {
            key: r.meta.key.clone(),
            user_key: r.user_key.clone(),
            name: r.name.clone(),
            plain_key: None,
            key_preview: r.key_preview.clone(),
            group: r.group.clone(),
            quota: r.quota,
            unlimited_quota: r.unlimited_quota,
            used_quota: r.used_quota,
            status: r.status,
            expires_at: r.expires_at.map(|t| t.format("%Y-%m-%d %H:%M").to_string()),
            // Record 无 created_at, Record→Dto 路径留空; wire 路径由后端填充。
            created_at: String::new(),
        }
    }
}

/// POST /api/token (创建) 请求体。
/// `quota`/`unlimited_quota` 后端有缺省（0 / false），前端可省略。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTokenRequest {
    /// 密钥名称（非空）。
    pub name: String,
    /// 分组别名；None = 跟随用户组。
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub quota: i64,
    #[serde(default)]
    pub unlimited_quota: bool,
    /// RFC3339；None = 永不过期。
    #[serde(default)]
    pub expires_at: Option<String>,
}

/// POST /api/token 响应：一次性明文 key + 密钥视图。
/// 明文只在创建响应出现一次，之后任何端点都只回 `key_preview`。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTokenResult {
    /// 一次性明文 `sk-…`（创建成功响应里出现一次）。
    #[serde(default)]
    pub plaintext: String,
    pub token: TokenDto,
}

/// PUT /api/token/{key} (编辑)。
/// 全 Option：缺省字段不随请求发出（`skip_serializing_if`），后端按「缺省=不改」处理。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTokenRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 注意：后端 group 是 `Option<Option<String>>`（None=不改，Some(None)=跟随用户组），
    /// 契约层不表达「跟随」语义，这里发 None = 不改。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unlimited_quota: Option<bool>,
    /// 1 启用 / 2 停用。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}
