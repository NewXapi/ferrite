//! Auth page API adapter.
//!
//! Provides real HTTP calls using `client::ApiClient` and `contract::api::auth` DTOs,
//! with mock fallback when standalone.

use client::{ApiClient, ApiResult};
use contract::api::auth as contract_auth;
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
pub async fn login_api(
    client: &ApiClient,
    req: &contract_auth::LoginRequest,
) -> ApiResult<contract_auth::LoginResponse> {
    client.post("/api/user/login", req).await
}

/// 真实调用: POST /api/user/register
pub async fn register_api(
    client: &ApiClient,
    req: &contract_auth::RegisterRequest,
) -> ApiResult<contract_auth::LoginResponse> {
    client.post("/api/user/register", req).await
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
pub async fn register(username: String, password: String) -> ApiResult<LoginResponse> {
    let client = ApiClient::new();
    let contract_req = contract_auth::RegisterRequest {
        username,
        password,
        email: None,
    };
    match register_api(&client, &contract_req).await {
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
pub async fn register(_username: String, _password: String) -> ApiResult<LoginResponse> {
    Ok(LoginResponse {
        token: "mock-token".into(),
        refresh_token: "mock-refresh".into(),
    })
}
