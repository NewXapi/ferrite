//! # api — Web/Console REST 契约 (前端 ↔ apps/console)
//!
//! 这里定义 Dioxus 前端直接消费的请求/响应形状。
//! 设计对齐两点:
//! 1. web 端 `crates/client` 已有的信封: `{"success": bool, "message": str, "data": ...}`;
//! 2. 字段从 `records.rs` 的 Record 经 `From` 投影, 前端永远不接触存储细节
//!    (如 SyncMeta.origin / key_hash)。
//!
//! 本模块只放"形状"; 每个端点的 URL、方法、鉴权要求在模块尾部的
//! `ENDPOINTS` 注释表中集中登记, 供 console 实现时逐条对照。

use serde::{Deserialize, Serialize};

use crate::records::{TokenRecord, UsageEventRecord, UserRecord};

/// 统一响应信封 — 与 web 端 `client::Envelope` 同构。
/// T 缺省 `serde_json::Value` 兼容旧端点渐进迁移。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

impl<T> Envelope<T> {
    pub fn ok(data: T) -> Self {
        Self { success: true, message: "ok".into(), data: Some(data) }
    }
    pub fn err(message: impl Into<String>) -> Self {
        Self { success: false, message: message.into(), data: None }
    }
}

// ---------------------------------------------------------------------------
// auth — 登录/会话 (对齐 page-auth 表单 + new-api /api/user/login)
// ---------------------------------------------------------------------------

/// POST /api/user/login
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 登录成功返回。access_token 短效, refresh_token 长效 (web 端已有一次性 401 刷新)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub user: UserDto,
    pub access_token: String,
    pub refresh_token: String,
    /// access_token 有效秒数, 前端据此安排静默刷新。
    pub expires_in: u64,
}

// ---------------------------------------------------------------------------
// user — 用户自读/自改
// ---------------------------------------------------------------------------

/// GET /api/user/self → data
/// 用户在前端的投影: 不含 role 内部位运算, 直接给语义化 role 名。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    pub key: String,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub quota: i64,
    pub used_quota: i64,
    pub request_count: u64,
    pub group: String,
    /// "user" | "admin" | "root" (从 u16 位映射, 转换逻辑见 From<UserRecord>)。
    pub role: String,
    pub status: u8,
    pub created_at: String,
}

impl From<&UserRecord> for UserDto {
    fn from(r: &UserRecord) -> Self {
        let role = match r.role {
            100 => "root",
            10 => "admin",
            _ => "user",
        };
        Self {
            key: r.meta.key.clone(),
            username: r.username.clone(),
            display_name: r.display_name.clone(),
            email: r.email.clone(),
            quota: r.quota,
            used_quota: r.used_quota,
            request_count: r.request_count,
            group: r.group.clone(),
            role: role.into(),
            status: r.status,
            created_at: r.created_at.format("%Y-%m-%d").to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// token — 密钥管理 (对齐 page-account keys 面板 + new-api /api/token/)
// ---------------------------------------------------------------------------

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
    pub group: String,
    pub quota: i64,
    pub unlimited_quota: bool,
    pub used_quota: i64,
    pub status: u8,
    pub expires_at: Option<String>,
}

impl From<&TokenRecord> for TokenDto {
    fn from(r: &TokenRecord) -> Self {
        let hash_tail = &r.key_hash[r.key_hash.len().saturating_sub(4)..];
        Self {
            key: r.meta.key.clone(),
            name: r.name.clone(),
            plain_key: None,
            masked_key: format!("sk-****{hash_tail}"),
            group: r.group.clone(),
            quota: r.quota,
            unlimited_quota: r.unlimited_quota,
            used_quota: 0, // TODO(#220): TokenRecord 缺 used_quota 字段, 要么加要么 join usage 汇总
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
    pub group: String,
    pub quota: i64,
    pub unlimited_quota: bool,
    pub expires_at: Option<String>,
}

// ---------------------------------------------------------------------------
// usage — 用量查询 (对齐 page-account usage_logs + page-overview)
// ---------------------------------------------------------------------------

/// GET /api/log/self?start=&end=&model=&p=&page_size= → data: UsageLogPage
/// 分页对齐 new-api: 传页码与页大小, 返回带总数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLogQuery {
    /// Unix 秒; 0 = 不限。
    pub start: i64,
    pub end: i64,
    /// 空字符串 = 不过滤。
    pub model: String,
    /// 1-based 页码。
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLogPage {
    pub items: Vec<UsageLogDto>,
    pub total: u64,
}

/// 前端日志行 — 与 mock::account::UsageLog 字段一一对齐 (前端已按此渲染)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLogDto {
    pub id: String,
    pub model: String,
    pub success: bool,
    /// Unix 秒。
    pub timestamp: i64,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub cached_tokens: u32,
    pub first_token_ms: u32,
    pub duration_ms: u32,
    /// 前端展示用金额 (美元, 保留 4 位)。
    pub cost: f64,
    pub error: Option<String>,
}

impl From<&UsageEventRecord> for UsageLogDto {
    fn from(e: &UsageEventRecord) -> Self {
        Self {
            id: e.meta.key.clone(),
            model: e.public_model.clone(),
            success: e.status_code >= 200 && e.status_code < 400,
            timestamp: e.meta.updated_at.timestamp(),
            prompt_tokens: e.prompt_tokens as u32,
            completion_tokens: e.completion_tokens as u32,
            cached_tokens: e.cached_tokens as u32,
            first_token_ms: e.first_token_ms,
            duration_ms: e.duration_ms,
            // 内部计费单位 → 美元: new-api 500_000 = $1
            cost: e.cost as f64 / 500_000.0,
            error: e.error.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// 端点登记表 (console 实现对照用; web client 按此发请求)
// ---------------------------------------------------------------------------
//
// | 方法 | 路径                      | 请求体              | data 类型        | 鉴权 |
// |------|---------------------------|---------------------|------------------|------|
// | POST | /api/user/login           | LoginRequest        | LoginResponse    | 无   |
// | GET  | /api/user/self            | -                   | UserDto          | AT   |
// | GET  | /api/token/               | -                   | Vec<TokenDto>    | AT   |
// | POST | /api/token/               | CreateTokenRequest  | TokenDto(含明文) | AT   |
// | DEL  | /api/token/{key}          | -                   | ()               | AT   |
// | GET  | /api/log/self             | UsageLogQuery(qs)   | UsageLogPage     | AT   |
//
// TODO(#221): models/overview/admin 三组端点在页面接入真后端前不定义,
//             字段以 crates/mock 对应面板当前消费的形状为蓝本再抽象。
// AT = Authorization: Bearer <access_token>
