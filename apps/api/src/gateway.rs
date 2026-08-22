//! HTTP 路由 + handler

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use futures_util::StreamExt;

use crate::adapter::{OpenAIAdapter, StreamResponse};
use crate::config::PgPool;
use crate::dispatch::RouteIndex;

/// 日志文件目录（main.rs tracing_appender 与 list_logs 共用）
pub const LOG_DIR: &str = "logs";

pub struct AppState {
    pub pool: PgPool,
    pub route_index: RouteIndex,
    pub adapter: OpenAIAdapter,
}

pub struct Gateway {
    state: Arc<AppState>,
}

impl Gateway {
    pub fn new(pool: PgPool, route_index: RouteIndex) -> Self {
        Self {
            state: Arc::new(AppState {
                pool,
                route_index,
                adapter: OpenAIAdapter::new(),
            }),
        }
    }

    pub fn router(&self) -> Router {
        use tower_http::trace::{DefaultOnResponse, TraceLayer};

        Router::new()
            .route("/health", get(health))
            .route("/v1/models", get(list_models))
            .route("/v1/chat/completions", post(chat_completions))
            .route("/admin/logs", get(list_logs))
            .layer(
                // 统一请求日志：span 声明业务字段，handler 内 record 填充，
                // TraceLayer 的 on_response 事件自动携带全部字段写入 JSON 日志文件
                TraceLayer::new_for_http()
                    .make_span_with(|req: &axum::http::Request<axum::body::Body>| {
                        tracing::info_span!(
                            "http_request",
                            method = %req.method(),
                            path = %req.uri().path(),
                            request_id = tracing::field::Empty,
                            user = tracing::field::Empty,
                            model = tracing::field::Empty,
                            channel = tracing::field::Empty,
                            upstream_model = tracing::field::Empty,
                            stream = tracing::field::Empty,
                            prompt_tokens = tracing::field::Empty,
                            completion_tokens = tracing::field::Empty,
                            total_tokens = tracing::field::Empty,
                        )
                    })
                    .on_response(DefaultOnResponse::new().level(tracing::Level::INFO)),
            )
            .with_state(self.state.clone())
    }
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

/// 构造 OpenAI 标准错误响应（application/json）
fn error_response(status: StatusCode, message: &str, error_type: &str) -> axum::response::Response {
    let body = serde_json::json!({
        "error": {
            "message": message,
            "type": error_type,
        }
    });
    (
        status,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json".to_string(),
        )],
        body.to_string(),
    )
        .into_response()
}

/// 认证失败状态码 → OpenAI error type
fn auth_error_type(status: StatusCode) -> &'static str {
    match status {
        StatusCode::UNAUTHORIZED => "invalid_api_key",
        StatusCode::FORBIDDEN => "permission_denied",
        StatusCode::PAYMENT_REQUIRED => "insufficient_quota",
        _ => "server_error",
    }
}

/// 统一认证：提取 token → PG 查询，错误已映射为 OpenAI 格式
async fn authenticate_request(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::identity::Pass, axum::response::Response> {
    let (status, message) = match crate::identity::extract_token(headers) {
        Ok(token) => match crate::identity::authenticate(&state.pool, &token).await {
            Ok(pass) => return Ok(pass),
            Err(e) => e,
        },
        Err(e) => e,
    };
    Err(error_response(status, &message, auth_error_type(status)))
}

/// GET /v1/models — 返回可用模型列表 (需要认证)
async fn list_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<axum::response::Response, axum::response::Response> {
    authenticate_request(&state, &headers).await?;

    let models = state.route_index.list_models();
    let data: Vec<serde_json::Value> = models
        .into_iter()
        .map(|id| serde_json::json!({"id": id, "object": "model"}))
        .collect();
    let body = serde_json::json!({"object": "list", "data": data});
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response())
}

/// POST /admin/reload — 从 PG 重新加载渠道配置 (仅 admin 组)
async fn reload_channels(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<axum::response::Response, axum::response::Response> {
    let pass = authenticate_request(&state, &headers).await?;
    crate::identity::require_admin(&pass).map_err(|(s, m)| error_response(s, &m, "permission_denied"))?;

    match load_channels(&state.pool).await {
        Ok(channels) => {
            let count = channels.len();
            state.route_index.build_from_channels(&channels);
            tracing::info!("reloaded {} channels", count);
            Ok((
                StatusCode::OK,
                [(
                    axum::http::header::CONTENT_TYPE,
                    "application/json".to_string(),
                )],
                serde_json::json!({"status": "ok", "channels": count}).to_string(),
            )
                .into_response())
        }
        Err(e) => {
            tracing::error!("failed to reload channels: {e}");
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &e.to_string(),
                "server_error",
            ))
        }
    }
}

/// POST /v1/chat/completions
///
/// 认证 → 解析 → 路由 → 限流 → 转发 (流式或非流式)
async fn chat_completions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<axum::response::Response, axum::response::Response> {
    // 1. 认证
    let pass = authenticate_request(&state, &headers).await?;
    let span = tracing::Span::current();
    span.record("request_id", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0).to_string());
    span.record("user", pass.username.as_str());

    // 2. 解析请求体
    let req: serde_json::Value = serde_json::from_slice(&body).map_err(|e| {
        error_response(
            StatusCode::BAD_REQUEST,
            &format!("invalid JSON: {e}"),
            "invalid_request_error",
        )
    })?;

    let model = req.get("model").and_then(|v| v.as_str()).ok_or_else(|| {
        error_response(
            StatusCode::BAD_REQUEST,
            "missing model field",
            "invalid_request_error",
        )
    })?;

    let is_stream = req.get("stream").and_then(|v| v.as_bool()).unwrap_or(false);
    let route = state.route_index.resolve(model).map_err(|e| {
        error_response(
            StatusCode::NOT_FOUND,
            &e.to_string(),
            "invalid_request_error",
        )
    })?;

    span.record("model", model);
    span.record("channel", route.channel_name.as_ref());
    span.record("upstream_model", route.upstream_model.as_ref());
    span.record("stream", is_stream);

    // 4. 限流（在路由解析之后，无效请求不消耗配额）
    crate::ratelimit::check_and_increment(&state.pool, &pass)
        .await
        .map_err(|(s, m)| error_response(s, &m, "rate_limit_reached"))?;

    // 5. 转发到上游
    if is_stream {
        let stream_resp = state
            .adapter
            .forward_stream(body, &route.base_url, &route.api_key, &route.upstream_model)
            .await
            .map_err(|e| {
                error_response(StatusCode::BAD_GATEWAY, &e.to_string(), "server_error")
            })?;

        Ok(stream_into_response(stream_resp).await?)
    } else {
        let resp = state
            .adapter
            .forward(body, &route.base_url, &route.api_key, &route.upstream_model)
            .await
            .map_err(|e| {
                error_response(StatusCode::BAD_GATEWAY, &e.to_string(), "server_error")
            })?;

        let status = StatusCode::from_u16(resp.status).unwrap_or(StatusCode::BAD_GATEWAY);

        // 解析 usage 记入请求 span（JSON 日志文件可见 token 数；流式为空）
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&resp.body)
            && let Some(u) = v.get("usage")
        {
            if let Some(n) = u.get("prompt_tokens").and_then(|x| x.as_i64()) {
                span.record("prompt_tokens", n);
            }
            if let Some(n) = u.get("completion_tokens").and_then(|x| x.as_i64()) {
                span.record("completion_tokens", n);
            }
            if let Some(n) = u.get("total_tokens").and_then(|x| x.as_i64()) {
                span.record("total_tokens", n);
            }
        }
        let mut response_headers = HeaderMap::new();
        if let Some(ct) = resp.content_type
            && let Ok(v) = ct.parse()
        {
            response_headers.insert(axum::http::header::CONTENT_TYPE, v);
        }

        Ok((status, response_headers, resp.body).into_response())
    }
}

/// 将 reqwest 流式响应转换为 axum Body 流
///
/// - 2xx 默认 text/event-stream；非 2xx 默认 application/json（上游错误体）
/// - 流中断时注入 SSE error frame，避免静默截断
async fn stream_into_response(resp: StreamResponse) -> Result<axum::response::Response, axum::response::Response> {
    let resp = crate::adapter::ensure_stream_ok(resp)
        .await
        .map_err(|e| error_response(StatusCode::BAD_GATEWAY, &e.to_string(), "upstream_error"))?;

    let status = StatusCode::from_u16(resp.status).unwrap_or(StatusCode::OK);

    let default_ct = if status.is_success() {
        "text/event-stream"
    } else {
        "application/json"
    };
    let content_type = resp
        .stream
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| default_ct.to_string());

    // 流错误 → 注入 SSE error frame 而非静默断开
    let stream = resp.stream.bytes_stream().map(|result| -> Result<bytes::Bytes, std::io::Error> {
        match result {
            Ok(bytes) => Ok(bytes),
            Err(e) => {
                tracing::warn!("upstream stream error: {e}");
                const FRAME: &str = "data: {\"error\":{\"message\":\"upstream stream interrupted\",\"type\":\"server_error\"}}\n\n";
                Ok(bytes::Bytes::from_static(FRAME.as_bytes()))
            }
        }
    });

    let body = Body::from_stream(stream);
    Ok((
        status,
        [
            (axum::http::header::CONTENT_TYPE, content_type),
            (axum::http::header::CACHE_CONTROL, "no-cache".to_string()),
        ],
        body,
    )
        .into_response())
}

/// 从 PG kv_store 加载渠道配置
pub async fn load_channels(
    pool: &sqlx::PgPool,
) -> Result<Vec<crate::dispatch::ChannelConfig>, sqlx::Error> {
    let rows: Vec<(String, serde_json::Value)> =
        sqlx::query_as("SELECT key, value FROM kv_store WHERE key LIKE 'channel:%'")
            .fetch_all(pool)
            .await?;

    Ok(rows
        .into_iter()
        .filter_map(|(_, value)| serde_json::from_value(value).ok())
        .collect())
}

/// F5.3 — GET /admin/logs 查询参数
#[derive(serde::Deserialize)]
pub struct LogQuery {
    pub user: Option<String>,
    pub model: Option<String>,
    pub channel: Option<String>,
    pub path: Option<String>,
    pub status: Option<u16>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// 展平 JSONL 日志行：合并 fields > span > spans[0] 为一个 flat map
fn flatten(v: &serde_json::Value) -> Option<serde_json::Value> {
    let obj = v.as_object()?;
    let mut merged = serde_json::Map::new();
    if let Some(arr) = obj.get("spans").and_then(|s| s.as_array())
        && let Some(first) = arr.first().and_then(|s| s.as_object())
    {
        for (k, val) in first {
            merged.insert(k.clone(), val.clone());
        }
    }
    if let Some(span) = obj.get("span").and_then(|s| s.as_object()) {
        for (k, val) in span {
            merged.insert(k.clone(), val.clone());
        }
    }
    if let Some(fields) = obj.get("fields").and_then(|s| s.as_object()) {
        for (k, val) in fields {
            merged.insert(k.clone(), val.clone());
        }
    }
    Some(serde_json::Value::Object(merged))
}

/// 纯函数：过滤 JSONL 行并分页
///
/// - 只保留"请求完成事件"行（顶层有 fields.status 的行）
/// - 展平 fields > span > spans[0]
/// - 字符串字段精确匹配；status 数值匹配；since/until 与 timestamp 做前缀比较
/// - 保持传入顺序；返回 (当前页数据, 匹配总数)
pub fn filter_log_lines(lines: &[&str], q: &LogQuery) -> (Vec<serde_json::Value>, usize) {
    let limit = q.limit.unwrap_or(50).clamp(1, 500);
    let offset = q.offset.unwrap_or(0);

    let mut matched: Vec<serde_json::Value> = Vec::new();
    for line in lines {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue; // 损坏行静默跳过
        };
        // 只保留请求完成事件行（顶层有 fields.status）
        if v.get("fields").and_then(|f| f.get("status")).is_none() {
            continue;
        }
        let Some(flat) = flatten(&v) else { continue };

        // 字符串字段精确匹配
        let str_match = |field: &str, val: &Option<String>| -> bool {
            val.as_ref().map_or(true, |want| {
                flat.get(field).and_then(|f| f.as_str()).map_or(false, |s| s == want)
            })
        };
        if !str_match("user", &q.user)
            || !str_match("model", &q.model)
            || !str_match("channel", &q.channel)
            || !str_match("path", &q.path)
        {
            continue;
        }

        // status 数值匹配
        if let Some(status) = q.status {
            if flat.get("status").and_then(|s| s.as_u64()) != Some(status as u64) {
                continue;
            }
        }

        // since/until 与顶层 timestamp 做前缀比较（RFC3339 字符串字典序 = 时间序）
        if let Some(ts) = v.get("timestamp").and_then(|t| t.as_str()) {
            if let Some(since) = &q.since {
                if ts < since.as_str() {
                    continue;
                }
            }
            if let Some(until) = &q.until {
                if ts > until.as_str() {
                    continue;
                }
            }
        }

        matched.push(flat);
    }

    let total = matched.len();
    let page = matched.into_iter().skip(offset).take(limit).collect();
    (page, total)
}

/// GET /admin/logs — 查询 JSONL 日志（仅 admin）
async fn list_logs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<LogQuery>,
) -> Result<axum::response::Response, axum::response::Response> {
    let pass = authenticate_request(&state, &headers).await?;
    crate::identity::require_admin(&pass)
        .map_err(|(s, m)| error_response(s, &m, "permission_denied"))?;

    // 收集日志行：文件名降序（新→旧），逐文件按行收集
    let mut all_lines: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(LOG_DIR) {
        let mut names: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|name| name.starts_with("ferrite.log."))
            .collect();
        names.sort_unstable_by(|a, b| b.cmp(a)); // 降序
        for name in names {
            if let Ok(content) = std::fs::read_to_string(std::path::Path::new(LOG_DIR).join(&name)) {
                for line in content.lines() {
                    all_lines.push(line.to_string());
                }
            }
        }
    }
    // 目录不存在或不可读 → 空结果，不报 500
    let refs: Vec<&str> = all_lines.iter().map(|s| s.as_str()).collect();
    let (data, total) = filter_log_lines(&refs, &q);

    let body = serde_json::json!({"object": "list", "total": total, "data": data});
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json".to_string())],
        body.to_string(),
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_error_type_maps_openai_types() {
        assert_eq!(auth_error_type(StatusCode::UNAUTHORIZED), "invalid_api_key");
        assert_eq!(auth_error_type(StatusCode::FORBIDDEN), "permission_denied");
        assert_eq!(
            auth_error_type(StatusCode::PAYMENT_REQUIRED),
            "insufficient_quota"
        );
        assert_eq!(
            auth_error_type(StatusCode::INTERNAL_SERVER_ERROR),
            "server_error"
        );
    }

    #[tokio::test]
    async fn error_response_is_json_with_openai_shape() {
        let resp = error_response(StatusCode::BAD_REQUEST, "bad", "invalid_request_error");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            resp.headers().get(axum::http::header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["error"]["message"], "bad");
        assert_eq!(v["error"]["type"], "invalid_request_error");
    }

    #[test]
    fn filter_log_lines_filters_and_paginates() {
        // 完成事件行（有 fields.status）
        let line1 = r#"{"timestamp":"2025-01-01T00:00:00Z","fields":{"status":200,"user":"alice","model":"gpt-4"},"span":{"channel":"ch1","path":"/v1/chat/completions"}}"#;
        // 非完成事件行（无 fields.status）→ 跳过
        let line2 = r#"{"timestamp":"2025-01-01T00:01:00Z","fields":{"user":"bob"}}"#;
        // status 不匹配
        let line3 = r#"{"timestamp":"2025-01-01T00:02:00Z","fields":{"status":500,"user":"alice"}}"#;
        // 损坏行 → 跳过
        let line4 = "not json at all";

        let lines = [line1, line2, line3, line4];

        // 无过滤：1 匹配（只有 line1 有 status）
        let q = LogQuery { user: None, model: None, channel: None, path: None, status: None, since: None, until: None, limit: None, offset: None };
        let (page, total) = filter_log_lines(&lines, &q);
        assert_eq!(total, 2); // line1 (200) + line3 (500)
        assert_eq!(page.len(), 2);

        // 按 user 过滤
        let q = LogQuery { user: Some("alice".into()), model: None, channel: None, path: None, status: None, since: None, until: None, limit: None, offset: None };
        let (page, total) = filter_log_lines(&lines, &q);
        assert_eq!(total, 2); // line1 + line3 都是 alice

        // 按 status 过滤
        let q = LogQuery { user: None, model: None, channel: None, path: None, status: Some(200), since: None, until: None, limit: None, offset: None };
        let (page, total) = filter_log_lines(&lines, &q);
        assert_eq!(total, 1);
        assert_eq!(page[0]["status"].as_u64(), Some(200));
        assert_eq!(page[0]["user"].as_str(), Some("alice"));
        assert_eq!(page[0]["channel"].as_str(), Some("ch1")); // 来自 span 展平

        // since 前缀比较
        let q = LogQuery { user: None, model: None, channel: None, path: None, status: None, since: Some("2025-01-01T00:01:30Z".into()), until: None, limit: None, offset: None };
        let (_, total) = filter_log_lines(&lines, &q);
        assert_eq!(total, 1); // 只有 line3 在 00:02:00

        // 分页
        let q = LogQuery { user: None, model: None, channel: None, path: None, status: None, since: None, until: None, limit: Some(1), offset: Some(1) };
        let (page, total) = filter_log_lines(&lines, &q);
        assert_eq!(total, 2); // 全量
        assert_eq!(page.len(), 1); // 第二页只取 1 条
    }
}
