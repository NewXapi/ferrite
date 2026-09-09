//! 用户端点 DTO。
//! 参考: mock::users::User 字段形状 + new-api /api/user/ 响应。

use crate::records::UserRecord;
use serde::{Deserialize, Serialize};

/// role 兼容反序列化：后端 UserView 以整数 (1/10/100) 返回，历史前端契约
/// 是语义字符串 ("user"/"admin"/"root")，两种都接受并归一为字符串。
fn de_role<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct RoleVisitor;
    impl serde::de::Visitor<'_> for RoleVisitor {
        type Value = String;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("role as integer 1/10/100 or string")
        }
        fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(match v {
                100 => "root".into(),
                10 => "admin".into(),
                _ => "user".into(),
            })
        }
        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(v.to_string())
        }
    }
    deserializer.deserialize_any(RoleVisitor)
}

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
    /// 后端 UserView 暂无此列,缺省 0 (2026-09-09 curl 实测)。
    #[serde(default)]
    pub request_count: u64,
    pub group: String,
    /// "user" | "admin" | "root";后端回整数 1/10/100 时自动归一。
    #[serde(deserialize_with = "de_role")]
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
