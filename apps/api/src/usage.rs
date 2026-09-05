//! Ferrite — 用量记录中间件
//!
//! 拦截 `/v1/*` POST 请求：
//! 1. Authorization Bearer → SHA-256 → token_snapshot 查找 TokenRecord
//! 2. TokenRecord.user_key → user_snapshot 查找 UserRecord（取 username）
//! 3. 缓存请求体（pipeline 本来就整包 to_bytes，无冲突）
//! 4. 响应体缓冲后解析 usage（JSON 直接取；SSE 扫 data 行取最后 usage）
//! 5. 落库：observe::logs::LogService::record(UsageEvent::consume)
//! 6. 副作用：api_tokens.used_quota += cost，quota_snapshot 扣除 cost
//!
//! 仅记录 2xx 响应；失败请求 TODO(#N) 占位。
//! channel_name/channel_key 在 pipeline 内部选定，本 PR 拿不到 → ponytail 注释说明。

use std::sync::Arc;
use std::time::Instant;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

use crate::snapshot::Snapshots;
use crate::PgPool;

/// 中间件共享状态
#[derive(Clone)]
pub struct UsageMiddlewareState {
    pub pool: PgPool,
    pub snapshots: Arc<Snapshots>,
}

/// 用量中间件入口
pub async fn usage_middleware(
    State(state): State<UsageMiddlewareState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if request.method() != axum::http::Method::POST || !request.uri().path().starts_with("/v1/") {
        return Ok(next.run(request).await);
    }

    let started = Instant::now();

    // 1. 提取并校验 Bearer
    let token_key = match extract_bearer(&request) {
        Some(t) => t,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    // 2. token_snapshot 查找（SHA-256 哈希）
    let token_snapshot = state.snapshots.token_snapshot.load();
    let hash_arr = match sha256_key(&token_key) {
        Some(h) => h,
        None => return Err(StatusCode::UNAUTHORIZED),
    };
    let token_entry = match token_snapshot.lookup(&hash_arr) {
        Some(e) => e,
        None => return Err(StatusCode::UNAUTHORIZED),
    };
    let user_key_str = token_entry.record.user_key.clone();
    let token_name = token_entry.record.name.clone();
    let token_uuid = match uuid::Uuid::parse_str(&token_entry.record.meta.key) {
        Ok(u) => u,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    // 3. user_snapshot 查找
    let user_snapshot = state.snapshots.user_snapshot.load();
    let username = match user_snapshot.lookup(&user_key_str) {
        Some(u) => u.username.clone(),
        None => return Err(StatusCode::UNAUTHORIZED),
    };
    let user_uuid = match uuid::Uuid::parse_str(&user_key_str) {
        Ok(u) => u,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    // 4. 缓存请求体
    let (parts, body) = request.into_parts();
    let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(b) => b,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };
    let model_name = extract_model(&body_bytes);

    // 5. 重构并执行请求
    let new_request = Request::from_parts(parts, body_bytes.clone().into());
    let response = next.run(new_request).await;

    if !response.status().is_success() {
        return Ok(response);
    }

    // 6. 缓冲响应体并解析 usage
    let (resp_parts, resp_body) = response.into_parts();
    let resp_bytes = match axum::body::to_bytes(resp_body, usize::MAX).await {
        Ok(b) => b,
        Err(_) => {
            return Ok(record_fallback(
                &state.pool, &state.snapshots.quota_snapshot, user_uuid, &username,
                token_uuid, &token_name, &model_name, &body_bytes, started, resp_parts,
            ));
        }
    };

    let (prompt_tokens, completion_tokens, is_stream) = parse_usage(&resp_bytes);
    let cost = (prompt_tokens + completion_tokens) as i64;
    let use_time_ms = started.elapsed().as_millis() as i32;

    spawn_record(
        state.pool.clone(),
        state.snapshots.quota_snapshot.clone(),
        user_uuid,
        username,
        token_uuid,
        token_name,
        model_name,
        prompt_tokens,
        completion_tokens,
        cost,
        use_time_ms,
        is_stream,
        token_entry.record.meta.key.clone(),
    );

    Ok(Response::from_parts(resp_parts, Body::from(resp_bytes)))
}

fn spawn_record(
    pool: PgPool, quota_snapshot: gateway_gate::snapshot::SharedQuota,
    user_uuid: uuid::Uuid, username: String, token_uuid: uuid::Uuid, token_name: String,
    model_name: String, prompt_tokens: i64, completion_tokens: i64, cost: i64,
    use_time_ms: i32, is_stream: bool, token_key: String,
) {
    tokio::spawn(async move {
        record_usage(
            &pool, &quota_snapshot, user_uuid, &username, token_uuid, &token_name,
            &model_name, prompt_tokens, completion_tokens, cost, use_time_ms, is_stream,
            &token_key,
        );
    });
}

fn record_fallback(
    pool: &PgPool, quota_snapshot: &gateway_gate::snapshot::SharedQuota,
    user_uuid: uuid::Uuid, username: &str, token_uuid: uuid::Uuid, token_name: &str,
    model_name: &str, body_bytes: &[u8], started: Instant, parts: axum::http::response::Parts,
) -> Response {
    let prompt = estimate_prompt_tokens(body_bytes);
    let cost = prompt as i64;
    let use_time_ms = started.elapsed().as_millis() as i32;
    spawn_record(
        pool.clone(), quota_snapshot.clone(), user_uuid, username.to_string(),
        token_uuid, token_name.to_string(), model_name.to_string(),
        prompt, 0, cost, use_time_ms, false, token_uuid.to_string(),
    );
    Response::from_parts(parts, Body::from(body_bytes.to_vec()))
}

async fn record_usage(
    pool: &PgPool, quota_snapshot: &gateway_gate::snapshot::SharedQuota,
    user_uuid: uuid::Uuid, username: &str, token_uuid: uuid::Uuid, token_name: &str,
    model_name: &str, prompt_tokens: i64, completion_tokens: i64, cost: i64,
    use_time_ms: i32, is_stream: bool, token_key: &str,
) {
    let event = observe::logs::UsageEvent {
        log_type: 1, // consume
        user_key: user_uuid,
        username: username.to_string(),
        token_key: Some(token_uuid),
        token_name: token_name.to_string(),
        channel_key: None,            // ponytail: pipeline 内部选定，本 PR 拿不到
        channel_name: String::new(),
        model_name: model_name.to_string(),
        prompt_tokens: prompt_tokens as i32,
        completion_tokens: completion_tokens as i32,
        quota: cost,
        use_time_ms,
        is_stream,
        ip: String::new(),
        request_id: String::new(),
        content: String::new(),
    };
    let svc = observe::logs::LogService::new(pool.clone());
    match svc.record(&event).await {
        Ok(id) => tracing::debug!(usage_id = %id, "usage recorded"),
        Err(e) => tracing::warn!(error = %e, "failed to record usage"),
    }
    if let Err(e) = sqlx::query("UPDATE api_tokens SET used_quota = used_quota + $1 WHERE key = $2")
        .bind(cost)
        .bind(token_key)
        .execute(pool)
        .await
    {
        tracing::warn!(error = %e, "failed to update used_quota");
    }
    quota_snapshot.load().add(token_key, -cost);
}

// ---- 辅助函数 ----

fn extract_bearer(request: &Request) -> Option<String> {
    request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| h.strip_prefix("Bearer ").unwrap().to_string())
}

fn sha256_key(token_key: &str) -> Option<[u8; 32]> {
    let bytes = hex::decode(token_key).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Some(arr)
}

fn extract_model(body_bytes: &[u8]) -> String {
    serde_json::from_slice::<serde_json::Value>(body_bytes)
        .ok()
        .and_then(|v| v.get("model").and_then(|m| m.as_str().map(String::from)))
        .unwrap_or_default()
}

fn parse_usage(body_bytes: &[u8]) -> (i64, i64, bool) {
    let text = String::from_utf8_lossy(body_bytes);
    let is_stream = text.contains("text/event-stream") || text.contains("data: ");
    if is_stream {
        let mut prompt = 0i64;
        let mut completion = 0i64;
        for line in text.lines() {
            let line = line.trim();
            if !line.starts_with("data:") {
                continue;
            }
            let json_str = line.trim_start_matches("data:").trim();
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                if let Some(usage) = v.get("usage") {
                    prompt = usage.get("prompt_tokens").and_then(|x| x.as_i64()).unwrap_or(prompt);
                    completion = usage.get("completion_tokens").and_then(|x| x.as_i64()).unwrap_or(completion);
                }
            }
        }
        return (prompt, completion, true);
    }
    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(body_bytes) {
        if let Some(usage) = v.get("usage") {
            let prompt = usage.get("prompt_tokens").and_then(|x| x.as_i64()).unwrap_or(0);
            let completion = usage.get("completion_tokens").and_then(|x| x.as_i64()).unwrap_or(0);
            return (prompt, completion, false);
        }
    }
    (0, 0, false)
}

fn estimate_prompt_tokens(body_bytes: &[u8]) -> i64 {
    let text = String::from_utf8_lossy(body_bytes);
    let mut chars = 0usize;
    for c in text.chars() {
        if c.is_ascii() || c.is_ascii_whitespace() {
            chars += 1;
        } else {
            chars += 3;
        }
    }
    (chars as f64 / 4.0).ceil() as i64
}
