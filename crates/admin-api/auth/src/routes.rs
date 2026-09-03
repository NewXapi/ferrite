//! axum 子路由 — POST /api/user/{login,register,refresh,logout}, GET /api/user/self。

use std::sync::Arc;

use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

use crate::error::AuthError;
use crate::service::AuthService;

#[derive(Clone)]
pub struct AppState {
    pub svc: Arc<AuthService>,
}

pub fn router(pool: PgPool) -> Result<Router, AuthError> {
    let secret = std::env::var("FERRITE_JWT_SECRET").map_err(|_| AuthError::MissingSecret)?;
    let svc = Arc::new(AuthService::new(pool, secret.into_bytes()));
    let state = AppState { svc };
    Ok(Router::new()
        .route("/api/user/login", post(login))
        .route("/api/user/register", post(register))
        .route("/api/user/refresh", post(refresh))
        .route("/api/user/logout", post(logout))
        .route("/api/user/self", get(self_view))
        .with_state(state))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterRequest {
    username: String,
    password: String,
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshRequest {
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct LogoutRequest {
    refresh_token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorBody {
    code: &'static str,
    message: String,
}

fn err_response(e: AuthError) -> (StatusCode, Json<ErrorBody>) {
    (
        e.status(),
        Json(ErrorBody {
            code: e.code(),
            message: e.to_string(),
        }),
    )
}

fn ua_ip(headers: &HeaderMap, ci: &ConnectInfo<std::net::SocketAddr>) -> (String, String) {
    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(',')
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    let ip = if ip.is_empty() {
        ci.0.ip().to_string()
    } else {
        ip
    };
    (ua, ip)
}

async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let (ua, ip) = ua_ip(&headers, &ConnectInfo(addr));
    match state.svc.login(&req.username, &req.password, &ua, &ip).await {
        Ok(r) => Ok(Json(json!(r))),
        Err(e) => Err(err_response(e)),
    }
}

async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let email = req.email.as_deref();
    match state.svc.register(&req.username, &req.password, email).await {
        Ok(u) => Ok(Json(json!(u))),
        Err(e) => Err(err_response(e)),
    }
}

async fn refresh(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let (ua, ip) = ua_ip(&headers, &ConnectInfo(addr));
    match state.svc.refresh(&req.refresh_token, &ua, &ip).await {
        Ok(r) => Ok(Json(json!(r))),
        Err(e) => Err(err_response(e)),
    }
}

async fn logout(
    State(state): State<AppState>,
    Json(req): Json<LogoutRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    match state.svc.logout(&req.refresh_token).await {
        Ok(()) => Ok(Json(json!({"success": true}))),
        Err(e) => Err(err_response(e)),
    }
}

async fn self_view(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or((
            StatusCode::UNAUTHORIZED,
            Json(ErrorBody {
                code: "MISSING_BEARER",
                message: "missing Authorization: Bearer ...".into(),
            }),
        ))?;
    match state.svc.self_by_access(token).await {
        Ok(u) => Ok(Json(json!(u))),
        Err(e) => Err(err_response(e)),
    }
}
