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
    let secret = std::env::var("FERRITE_JWT_SECRET").unwrap_or_else(|_| "dev_ferrite_jwt_secret_key_32bytes_len!".into());
    let svc = Arc::new(AuthService::new(pool, secret.into_bytes())?);
    router_with_svc(svc)
}

/// 共享 AuthService 的组装入口 — admin-api-router 聚合多个子域时用。
pub fn router_with_svc(svc: Arc<AuthService>) -> Result<Router, AuthError> {
    Ok(Router::new()
        .route("/api/user/login", post(login))
        .route("/api/user/register", post(register))
        .route("/api/user/refresh", post(refresh))
        .route("/api/user/logout", post(logout))
        .route(
            "/api/user/self",
            get(self_view).put(update_self).delete(delete_self),
        )
        .route("/api/user", get(list_users))
        .route("/api/user/search", get(search_users))
        .route("/api/user/{key}", get(get_user))
        .route("/api/user/manage", post(manage_user))
        .with_state(AppState { svc }))
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
    match state
        .svc
        .login(&req.username, &req.password, &ua, &ip)
        .await
    {
        Ok(r) => Ok(Json(json!(r))),
        Err(e) => Err(err_response(e)),
    }
}

async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let email = req.email.as_deref();
    match state
        .svc
        .register(&req.username, &req.password, email)
        .await
    {
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

// ---------- Bearer 解析 + admin 守卫 ----------

/// 从 Authorization header 提取 Bearer 并校验 → UserView。
/// catalog/token 等兄弟域复用。
pub async fn bearer_user(
    svc: &AuthService,
    headers: &HeaderMap,
) -> Result<crate::service::UserView, AuthError> {
    let token = bearer_token(headers)?;
    svc.self_by_access(token).await
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, AuthError> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(AuthError::InvalidToken)
}

/// role >= 10 即管理权限 (10=admin, 100=root; 1=普通用户)。
pub const ADMIN_ROLE_THRESHOLD: u16 = 10;

fn require_admin(user: &crate::service::UserView) -> Result<(), AuthError> {
    if user.role >= ADMIN_ROLE_THRESHOLD {
        Ok(())
    } else {
        Err(AuthError::Forbidden)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateSelfRequest {
    display_name: Option<String>,
    original_password: Option<String>,
    new_password: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManageUserRequest {
    key: String,
    action: ManageUserAction,
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ManageUserAction {
    Enable,
    Disable,
    SetRole,
    AdjustQuota,
    ResetPassword,
}

impl ManageUserAction {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Enable => "enable",
            Self::Disable => "disable",
            Self::SetRole => "set_role",
            Self::AdjustQuota => "adjust_quota",
            Self::ResetPassword => "reset_password",
        }
    }
}

async fn update_self(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpdateSelfRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let user = bearer_user(&state.svc, &headers)
        .await
        .map_err(err_response)?;
    match state
        .svc
        .update_self(
            uuid::Uuid::parse_str(&user.key)
                .map_err(|_| AuthError::InvalidToken)
                .map_err(err_response)?,
            req.display_name.as_deref(),
            req.original_password.as_deref(),
            req.new_password.as_deref(),
        )
        .await
    {
        Ok(u) => Ok(Json(json!(u))),
        Err(e) => Err(err_response(e)),
    }
}

async fn delete_self(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let user = bearer_user(&state.svc, &headers)
        .await
        .map_err(err_response)?;
    let key = uuid::Uuid::parse_str(&user.key)
        .map_err(|_| AuthError::InvalidToken)
        .map_err(err_response)?;
    match state.svc.delete_user(key).await {
        Ok(()) => Ok(Json(json!({"success": true}))),
        Err(e) => Err(err_response(e)),
    }
}

async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<ListQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let user = bearer_user(&state.svc, &headers)
        .await
        .map_err(err_response)?;
    require_admin(&user).map_err(err_response)?;
    match state
        .svc
        .list_users(
            q.search.as_deref(),
            q.page.unwrap_or(1),
            q.size.unwrap_or(20),
        )
        .await
    {
        Ok((users, total)) => Ok(Json(json!({"items": users, "total": total}))),
        Err(e) => Err(err_response(e)),
    }
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    search: Option<String>,
    page: Option<i64>,
    size: Option<i64>,
}

async fn manage_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ManageUserRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let user = bearer_user(&state.svc, &headers)
        .await
        .map_err(err_response)?;
    require_admin(&user).map_err(err_response)?;
    let key = uuid::Uuid::parse_str(&req.key)
        .map_err(|_| AuthError::BadRequest("invalid user key".into()))
        .map_err(err_response)?;
    match state
        .svc
        .manage_user(key, req.action.as_str(), req.value.as_deref())
        .await
    {
        Ok(u) => Ok(Json(json!(u))),
        Err(e) => Err(err_response(e)),
    }
}

/// GET /api/user/search?keyword= — admin 搜索（前 20 条）。
async fn search_users(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let user = bearer_user(&state.svc, &headers)
        .await
        .map_err(err_response)?;
    require_admin(&user).map_err(err_response)?;
    match state
        .svc
        .search_users(q.get("keyword").map(String::as_str).unwrap_or(""))
        .await
    {
        Ok(items) => Ok(Json(json!({ "items": items }))),
        Err(e) => Err(err_response(e)),
    }
}

/// GET /api/user/{key} — admin 单查。
async fn get_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let user = bearer_user(&state.svc, &headers)
        .await
        .map_err(err_response)?;
    require_admin(&user).map_err(err_response)?;
    let key = uuid::Uuid::parse_str(&key)
        .map_err(|_| AuthError::BadRequest("invalid user key".into()))
        .map_err(err_response)?;
    match state.svc.get_user(key).await {
        Ok(u) => Ok(Json(json!(u))),
        Err(e) => Err(err_response(e)),
    }
}
