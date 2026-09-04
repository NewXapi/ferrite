//! 登录/会话端点 DTO。
//! 参考: page-auth 表单字段 + new-api /api/user/login + client crate 的一次性 401 刷新。

use crate::records::UserRecord;
use crate::records::identity::TokenRecord;
use serde::{Deserialize, Serialize};

use super::user::UserDto;

/// POST /api/user/login
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// POST /api/user/register
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub email: Option<String>,
}
/// 登录成功返回。access_token 短效, refresh_token 长效 (web 端已有一次性 401 刷新)。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub user: UserDto,
    pub access_token: String,
    pub refresh_token: String,
    /// access_token 有效秒数, 前端据此安排静默刷新。
    pub expires_in: u64,
}

/// POST /api/user/reset — 找回密码申请。
/// TODO(#205): 邮件验证码流程 — console 邮件通道 (ops::notify) 就绪后启用。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordRequest {
    pub email: String,
}

/// DELETE /api/user/logout — 吊销 refresh token (服务端拉黑)。
/// TODO(#208): refresh token 轮换 + 拉黑表 — 需要存储支持 (identity 域)。
pub struct LogoutRequest {
    pub refresh_token: String,
}

/// 测试辅助: 从记录构建登录响应 (console 单测用)。
pub fn login_response_from(_user: &UserRecord, _tokens: &[TokenRecord]) -> LoginResponse {
    todo!("TODO(#205): console 实现时提供真实构造")
}
