//! 用户端点 DTO。
//! 参考: mock::users::User 字段形状 + new-api /api/user/ 响应。

use crate::records::UserRecord;
use serde::{Deserialize, Serialize};

/// GET /api/user/self → data
/// 用户在前端的投影: 不含密码哈希等存储细节, role 转语义化字符串。
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
    /// "user" | "admin" | "root" (从 u16 位映射, 转换逻辑见 From<&UserRecord>)。
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

/// PUT /api/user/self — 用户自改资料 (显示名/邮箱; 密码走独立端点)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileRequest {
    pub display_name: String,
    pub email: String,
}

/// PUT /api/user/password
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}
