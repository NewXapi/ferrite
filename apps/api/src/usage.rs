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
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;

use crate::PgPool;
use crate::snapshot::Snapshots;

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
                &state,
                RecordJob {
                    user_uuid,
                    username,
                    token_uuid,
                    token_name,
                    model_name,
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    cost: 0,
                    use_time_ms: started.elapsed().as_millis() as i32,
                    is_stream: false,
                    token_key: token_entry.record.meta.key.clone(),
                },
                &body_bytes,
                resp_parts,
            ));
        }
    };

    let (prompt_tokens, completion_tokens, is_stream) = parse_usage(&resp_bytes);
    let cost = prompt_tokens + completion_tokens;
    let use_time_ms = started.elapsed().as_millis() as i32;

    spawn_record(
        state.pool.clone(),
        state.snapshots.quota_snapshot.clone(),
        RecordJob {
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
            token_key: token_entry.record.meta.key.clone(),
        },
    );

    Ok(Response::from_parts(resp_parts, Body::from(resp_bytes)))
}

/// 一次用量记录的全部载荷（打包成 struct 避免 13 参数函数）。
///
/// 对外公开只为让集成测试能直接喂 [`build_consume_event`]，不作为稳定 API。
pub struct RecordJob {
    pub user_uuid: uuid::Uuid,
    pub username: String,
    pub token_uuid: uuid::Uuid,
    pub token_name: String,
    pub model_name: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cost: i64,
    pub use_time_ms: i32,
    pub is_stream: bool,
    pub token_key: String,
}

/// 把一次请求的用量载荷翻译成 observe 的 [`observe::logs::UsageEvent`]。
///
/// `log_type` 由 [`observe::logs::UsageEvent::consume`] 构造函数内部设为
/// [`observe::logs::LOG_TYPE_CONSUME`]（= 2），本函数不得手写字面量：观测侧的排行榜
/// `/api/log/top` 与趋势 `/api/log/trend` 都按 `log_type = 2` 过滤，这里曾手写
/// `log_type: 1`（1=充值，见 `db/migrations/0002_usage_logs.sql`），
/// 导致每条真实消费都被记成充值并从两个总览查询里整体消失。
///
/// `channel_key` / `channel_name` 沿用构造器默认的空值：渠道在 pipeline 内部选定，
/// 中间件这一层拿不到（ponytail，待 pipeline 回传选中渠道后补）。
/// `ip` / `request_id` / `content` 同理留空。
pub fn build_consume_event(job: &RecordJob) -> observe::logs::UsageEvent {
    let mut event =
        observe::logs::UsageEvent::consume(job.user_uuid, &job.username, &job.model_name);
    event.token_key = Some(job.token_uuid);
    event.token_name = job.token_name.clone();
    // usage_logs 的 token 列是 i32（0002 迁移）；clamp 而非裸 as，异常大的计数
    // 截到 i32::MAX 而不是回绕成负数污染 sum 聚合。
    event.prompt_tokens = job.prompt_tokens.clamp(0, i32::MAX as i64) as i32;
    event.completion_tokens = job.completion_tokens.clamp(0, i32::MAX as i64) as i32;
    event.quota = job.cost;
    event.use_time_ms = job.use_time_ms;
    event.is_stream = job.is_stream;
    event
}

fn spawn_record(
    mut pool: PgPool,
    mut quota_snapshot: gateway_gate::snapshot::SharedQuota,
    job: RecordJob,
) {
    tokio::spawn(async move {
        record_usage(&pool, &quota_snapshot, job).await;
        let _ = (&mut pool, &mut quota_snapshot); // 保持所有权到 spawn 结束
    });
}

fn record_fallback(
    state: &UsageMiddlewareState,
    job: RecordJob,
    body_bytes: &[u8],
    parts: axum::http::response::Parts,
) -> Response {
    let prompt = estimate_prompt_tokens(body_bytes);
    spawn_record(
        state.pool.clone(),
        state.snapshots.quota_snapshot.clone(),
        RecordJob {
            prompt_tokens: prompt,
            cost: prompt,
            ..job
        },
    );
    Response::from_parts(parts, Body::from(body_bytes.to_vec()))
}

async fn record_usage(
    pool: &PgPool,
    quota_snapshot: &gateway_gate::snapshot::SharedQuota,
    job: RecordJob,
) {
    let event = build_consume_event(&job);
    let RecordJob {
        cost, token_key, ..
    } = job;
    let svc = observe::logs::LogService::new(pool.clone());
    match svc.record(&event).await {
        Ok(id) => tracing::debug!(usage_id = %id, "usage recorded"),
        Err(e) => tracing::warn!(error = %e, "failed to record usage"),
    }
    // api_tokens.key 是 UUID 列：必须绑 Uuid，绑 String 会类型不匹配导致 0 行更新
    match uuid::Uuid::parse_str(&token_key) {
        Ok(key_uuid) => {
            if let Err(e) =
                sqlx::query("UPDATE api_tokens SET used_quota = used_quota + $1 WHERE key = $2")
                    .bind(cost)
                    .bind(key_uuid)
                    .execute(pool)
                    .await
            {
                tracing::warn!(error = %e, "failed to update used_quota");
            }
        }
        Err(e) => tracing::warn!(error = %e, token_key = %token_key, "token key is not a uuid"),
    }
    // DB 用 UUID 主键，内存 quota 快照桶键同样是 token 的 UUID 字符串
    // （与 QuotaGate 查询键 TokenInfo.id 一致，见 snapshot::build_quota_snapshot）
    quota_snapshot.load().add(&token_key, -cost);
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

/// AuthGate 用 sha256(明文 key) 查 TokenSnapshot，这里必须一致：
/// bearer 是 `sk-<random>` 明文而非 hex，早期误用 hex::decode 导致中间件恒 401。
fn sha256_key(token_key: &str) -> Option<[u8; 32]> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token_key.as_bytes());
    Some(hasher.finalize().into())
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
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str)
                && let Some(usage) = v.get("usage")
            {
                prompt = usage
                    .get("prompt_tokens")
                    .and_then(|x| x.as_i64())
                    .unwrap_or(prompt);
                completion = usage
                    .get("completion_tokens")
                    .and_then(|x| x.as_i64())
                    .unwrap_or(completion);
            }
        }
        return (prompt, completion, true);
    }
    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(body_bytes)
        && let Some(usage) = v.get("usage")
    {
        let prompt = usage
            .get("prompt_tokens")
            .and_then(|x| x.as_i64())
            .unwrap_or(0);
        let completion = usage
            .get("completion_tokens")
            .and_then(|x| x.as_i64())
            .unwrap_or(0);
        return (prompt, completion, false);
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
