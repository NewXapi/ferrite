//! 用户端点 DTO。
//! 参考: mock::users::User 字段形状 + new-api /api/user/ 响应。

use crate::records::UserRecord;
use serde::{Deserialize, Serialize};

/// GET /api/user/self → data
/// 用户在前端的投影: 不含密码哈希等存储细节, role 转语义化字符串。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    pub key: String,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub quota: i64,
    pub used_quota: i64,
    /// auth_users 表无该列, 后端登录/self 响应可能不含此字段, 缺省为 0 以兼容。
    #[serde(default)]
    pub request_count: u64,
    pub group: String,
    /// "user" | "admin" | "root"。后端 wire 上是整数 role (1/10/100),
    /// 这里兼容整数与字符串两种形态, 统一归一成语义字符串。
    #[serde(deserialize_with = "de_role", default = "default_role")]
    pub role: String,
    pub status: u8,
    pub created_at: String,
}

/// 后端 `UserView.role` 是 SMALLINT (1 | 10 | 100), 但本 DTO 约定语义字符串。
/// 同时容忍已经序列化为字符串的调用方, 保证登录/self/账户页都能解析成功。
fn de_role<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Deserialize;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum RawRole {
        Str(String),
        Num(u32),
    }
    match RawRole::deserialize(d)? {
        RawRole::Str(s) => Ok(s),
        RawRole::Num(n) => Ok(match n {
            100 => "root",
            10 => "admin",
            _ => "user",
        }
        .to_string()),
    }
}

fn default_role() -> String {
    "user".to_string()
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
