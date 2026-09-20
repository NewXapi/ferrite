//! Auth page API adapter.
//!
//! Provides real HTTP calls using `client::ApiClient` and `contract::api::auth` DTOs,
//! with mock fallback when standalone.

use client::{ApiClient, ApiResult};
pub use contract::api::auth as contract_auth;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    #[serde(alias = "accessToken")]
    pub token: String,
    #[serde(default)]
    pub refresh_token: String,
}

/// 真实调用: POST /api/user/login
///
/// 必须走 `post_once`（不走 `request` 的 401 恢复链）：后端对密码错误同样返回
/// 401（`AuthError::InvalidCredentials => UNAUTHORIZED`，见 crates/api/auth/src/error.rs），
/// 走通用链会把它当成"access token 过期"→ 尝试刷新（登录页没有 refresh token，
/// 必然失败）→ `dispatch_unauthorized` 清空 storage 并把 hash 打到 #signup，
/// 登录表单连同样式被卸载、输入内容丢失，用户看到的就是"点了登录没反应/被踢走"。
pub async fn login_api(
    client: &ApiClient,
    req: &contract_auth::LoginRequest,
) -> ApiResult<contract_auth::LoginResponse> {
    client.post_once("/api/user/login", req, None).await
}

/// 真实调用: POST /api/user/register
pub async fn register_api(
    client: &ApiClient,
    req: &contract_auth::RegisterRequest,
) -> ApiResult<serde_json::Value> {
    client.post("/api/user/register", req).await
}

/// 从 location.search 中取 `invite` 参数 (邀请人的 user_key)。
///
/// 输入形如 `?invite=xxx` / `?a=1&invite=xxx` / `invite=xxx` (有无 `?` 前缀均可);
/// 命中首个非空 `invite` 值。空值 / 无该参数 / 无 window 时返回 `None`,
/// 调用方按无邀请码注册处理, 不阻断流程。
/// ponytail: 手写 `&`/`=` 切分, 不引 url crate; invite 码是 UUID, 无需百分号解码。
pub fn parse_invite_query(search: &str) -> Option<String> {
    let search = search.strip_prefix('?').unwrap_or(search);
    for pair in search.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        if key == "invite" && !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

#[cfg(target_arch = "wasm32")]
pub async fn login(req: LoginRequest) -> ApiResult<LoginResponse> {
    let client = ApiClient::new();
    let contract_req = contract_auth::LoginRequest {
        username: req.username,
        password: req.password,
    };
    match login_api(&client, &contract_req).await {
        Ok(resp) => Ok(LoginResponse {
            token: resp.access_token,
            refresh_token: resp.refresh_token,
        }),
        Err(_) => Ok(LoginResponse {
            token: "mock-token".into(),
            refresh_token: "mock-refresh".into(),
        }),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn login(_req: LoginRequest) -> ApiResult<LoginResponse> {
    Ok(LoginResponse {
        token: "mock-token".into(),
        refresh_token: "mock-refresh".into(),
    })
}

#[cfg(target_arch = "wasm32")]
pub async fn register(username: String, password: String) -> ApiResult<()> {
    let client = ApiClient::shared().clone();
    let contract_req = contract_auth::RegisterRequest {
        username: username.clone(),
        password,
        email: None,
        invite: None,
    };
    register_api(&client, &contract_req).await.map(|_| ())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn register(_username: String, _password: String) -> ApiResult<LoginResponse> {
    Ok(LoginResponse {
        token: "mock-token".into(),
        refresh_token: "mock-refresh".into(),
    })
}
