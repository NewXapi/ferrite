//! API key 认证：从 PG 查 token，构造 Pass

use crate::config::PgPool;
use axum::http::{HeaderMap, StatusCode};

/// 认证后的用户快照
#[derive(Debug, Clone)]
#[allow(dead_code)] // quota/billing 相关字段，后续计费会用
pub struct Pass {
    pub token_key: String,
    pub user_id: i64,
    pub username: String,
    pub quota: i64,
    pub used_quota: i64,
    pub group: String,
}

/// 从 Authorization header 提取 Bearer token
pub fn extract_token(headers: &HeaderMap) -> Result<String, (StatusCode, String)> {
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "missing Authorization header".into(),
        ))?;

    let token = auth
        .strip_prefix("Bearer ")
        .or_else(|| auth.strip_prefix("bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "expected Bearer token".into()))?;

    Ok(token.trim().to_string())
}

/// 从 PG 查 token，返回 Pass
pub async fn authenticate(pool: &PgPool, token: &str) -> Result<Pass, (StatusCode, String)> {
    let row: Option<(i64, String, i64, i64, String, bool)> = sqlx::query_as(
        r#"SELECT user_id, username, quota, used_quota, "group", enabled FROM tokens WHERE key = $1"#,
    )
    .bind(token)
    .fetch_optional(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?;

    let row = row.ok_or((StatusCode::UNAUTHORIZED, "invalid token".into()))?;

    if !row.5 {
        return Err((StatusCode::FORBIDDEN, "token disabled".into()));
    }

    let remaining = row.2 - row.3;
    if remaining <= 0 {
        return Err((StatusCode::PAYMENT_REQUIRED, "insufficient quota".into()));
    }

    Ok(Pass {
        token_key: token.to_string(),
        user_id: row.0,
        username: row.1,
        quota: row.2,
        used_quota: row.3,
        group: row.4,
    })
}
