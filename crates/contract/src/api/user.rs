//! 用户端点 DTO。
//! 参考: mock::users::User 字段形状 + new-api /api/user/ 响应。

use crate::records::UserRecord;
use serde::{Deserialize, Serialize};

/// GET /api/user/self → 用户在前端的投影。
///
/// 字段与后端 `auth::service::UserView` 的 wire 形状一一对齐 (camelCase)：
/// 不含密码哈希等存储细节。`role` 是 wire 上的 u16 位值 (1/10/100)，
/// 语义化标签走 [`role_label`]；`request_count` 真实 `/self` 不返回故可选；
/// `auth_version` 改密自增, Record→Dto 路径不填充 (默认 0)。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    pub key: String,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub quota: i64,
    pub used_quota: i64,
    /// 请求计数。真实 `/self` (UserView) 不带此字段, 故 `Option` + `#[serde(default)]`。
    #[serde(default)]
    pub request_count: Option<u64>,
    pub group: String,
    /// 1=user | 10=admin | 100=root (wire 位值, 语义化用 [`role_label`])。
    pub role: u16,
    pub status: u8,
    /// 改密自增, 用于让旧 access/refresh 失效。Record→Dto 路径默认 0。
    #[serde(default)]
    pub auth_version: i64,
    pub created_at: String,
}

impl From<&UserRecord> for UserDto {
    fn from(r: &UserRecord) -> Self {
        Self {
            key: r.meta.key.clone(),
            username: r.username.clone(),
            display_name: r.display_name.clone(),
            email: r.email.clone(),
            quota: r.quota,
            used_quota: r.used_quota,
            request_count: Some(r.request_count),
            group: r.group.clone(),
            role: r.role,
            status: r.status,
            auth_version: 0,
            created_at: r.created_at.format("%Y-%m-%d").to_string(),
        }
    }
}

/// role 位值 → 语义化标签: 100→root, 10→admin, 其余→user。
pub fn role_label(role: u16) -> &'static str {
    match role {
        100 => "root",
        10 => "admin",
        _ => "user",
    }
}

/// GET /api/user/self/sessions → 单条会话。
///
/// 后端 `auth::service::SessionView` 的 wire 形状；时间字段由后端序列化为
/// RFC3339 字符串, 前端 wire 用 `String` 承载, 避免 wasm 侧引入 chrono。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto {
    pub sid: String,
    pub user_agent: String,
    pub ip: String,
    pub login_method: String,
    pub created_at: String,
    pub last_active: String,
    pub expires_at: String,
    /// 是否当前设备会话 (后端按 access token 里的 sid 标注)。
    pub current: bool,
}

/// PUT /api/user/self — 改显示名 / 改密码 (改密须原密码 + 新密码成对提供)。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSelfRequest {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub original_password: Option<String>,
    #[serde(default)]
    pub new_password: Option<String>,
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
