//! 身份记录 — 用户与 API 令牌。
//!
//! 参考: mock::users::User 字段形状, new-api internal/identity, one-api user/token 表,
//! 07-database-schema.md 域二。

use super::SyncMeta;
use serde::{Deserialize, Serialize};

/// 用户。
///
/// `quota` 沿用 new-api 语义 (内部计费最小单位, 500_000 = $1)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRecord {
    pub meta: SyncMeta,
    pub username: String,
    pub display_name: String,
    pub email: String,
    /// 剩余额度 (内部单位)。
    pub quota: i64,
    /// 已消耗额度。
    pub used_quota: i64,
    pub request_count: u64,
    pub group: String,
    /// 1 启用 / 2 禁用 (对齐 new-api)。
    pub status: u8,
    /// 1 用户 / 10 管理员 / 100 root。
    pub role: u16,
    /// 用户所属组别名变更历史 (TODO(#207): auto-group 策略 — new-api resolve_group.go)。
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 前端/客户端调用 API 用的令牌 (sk-...)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenRecord {
    pub meta: SyncMeta,
    pub user_key: String, // 指向 UserRecord::meta.key
    pub name: String,
    /// SHA-256(明文 key)。明文只在创建响应里出现一次, 契约层永不存明文。
    pub key_hash: String,
    /// 掩码预览 "sk-ab****ef" (列表展示用, 不需要解密)。
    pub key_preview: String,
    /// 令牌固定分组; None = 跟随用户组 (new-api token group 语义)。
    pub group: Option<String>,
    /// 令牌独立限额; unlimited_quota = true 时忽略。
    pub quota: i64,
    pub unlimited_quota: bool,
    pub used_quota: i64,
    /// 可选过期时间。
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub status: u8,
}
