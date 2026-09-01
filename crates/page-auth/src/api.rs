//! Auth page private API adapter.
//!
//! Currently returns mock data so the page can render without a backend.
//! Once the backend endpoints exist, swap the mock bodies for HTTP calls.

use client::ApiResult;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoginResponse {
    pub token: String,
}

pub async fn login(_req: LoginRequest) -> ApiResult<LoginResponse> {
    // TODO: replace with HTTP POST /api/auth/login
    Ok(LoginResponse {
        token: "mock-token".into(),
    })
}

pub async fn register(_username: String, _password: String) -> ApiResult<LoginResponse> {
    // TODO: replace with HTTP POST /api/auth/register
    Ok(LoginResponse {
        token: "mock-token".into(),
    })
}
