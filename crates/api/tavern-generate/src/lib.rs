//! tavern-generate — 生成请求转发 + SSE 透传 + 中止
//!
//! 对标 SillyTavern `src/endpoints/backends/chat-completions.js`：
//! 不读角色卡、不读聊天文件，只取密钥并转发前端拼好的 OpenAI body。

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use serde_json::json;
use tavern_secrets::SecretError;
use tavern_storage::UserDirs;

/// 生成转发所需的运行时配置。
#[derive(Clone)]
pub struct GenerateConfig {
    /// OpenAI 兼容上游，通常是本进程的 `/v1` 或外部 gateway。
    pub upstream: String,
}

impl Default for GenerateConfig {
    fn default() -> Self {
        Self {
            upstream: "http://127.0.0.1:3000".into(),
        }
    }
}

#[derive(Clone)]
pub struct GenerateState {
    pub dirs: UserDirs,
    pub config: GenerateConfig,
    pub http: reqwest::Client,
}

impl GenerateState {
    pub fn new(dirs: UserDirs, config: GenerateConfig) -> Self {
        Self {
            dirs,
            config,
            http: reqwest::Client::new(),
        }
    }
}

pub fn router(state: GenerateState) -> Router {
    Router::new()
        .route("/generate", post(generate))
        .route("/status", get(status))
        .with_state(Arc::new(state))
}

async fn status() -> impl IntoResponse {
    Json(json!({ "ok": true }))
}

async fn generate(
    State(st): State<Arc<GenerateState>>,
    headers: HeaderMap,
    body: BytesBody,
) -> Response {
    // R3 后端校验：含 `_ferrite_agent_prompt_marker` 的 payload 必须已物化。
    // 非 marker payload 一律不校验，行为不变。
    let mut bytes = body.0;
    if let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(&bytes)
        && value.as_object().is_some_and(|obj| {
            obj.contains_key("_ferrite_agent_prompt_marker")
                || obj.contains_key("_tauritavern_agent_prompt_marker")
        })
    {
        if let Err(e) = harness_prompt::reject_unfinalized_snapshot(&value) {
            return bad_request(&e.to_string());
        }
        // marker 纯内部协议字段，必须摘除再转发，
        // 否则上游(OpenAI-compatible)会因未知参数 400。
        strip_prompt_marker(&mut value);
        if let Ok(rest) = serde_json::to_vec(&value) {
            bytes = bytes::Bytes::from(rest);
        }
    }

    let key = match tavern_secrets::read(&st.dirs.secrets_file(), "api_key_openai") {
        Ok(k) => k,
        Err(SecretError::Storage(_) | SecretError::Json(_)) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "secrets").into_response();
        }
    };
    let url = format!(
        "{}/v1/chat/completions",
        st.config.upstream.trim_end_matches('/')
    );
    let mut req = st
        .http
        .post(url)
        .body(bytes)
        .header("content-type", "application/json");
    if let Some(k) = key {
        req = req.bearer_auth(k);
    }
    if let Some(accept) = headers.get("accept") {
        req = req.header("accept", accept);
    }
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": {"message": e.to_string(), "type": "upstream"}})),
            )
                .into_response();
        }
    };
    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let mut out = Response::builder().status(status);
    if let Some(ct) = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
    {
        out = out.header("content-type", ct);
    }
    use futures_util::StreamExt;
    let stream = resp
        .bytes_stream()
        .map(|r| r.map_err(std::io::Error::other));
    out.body(Body::from_stream(stream))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// 摘除 prompt marker 字段（新旧两个别名），防止其泄漏到上游请求。
///
/// marker 是 Ferrite業内协议字段：前端置 `_ferrite_agent_prompt_marker: ""` 表示已物化。
/// 转发前移除，避免 OpenAI-compatible 上游因未知参数拒收。
pub fn strip_prompt_marker(value: &mut serde_json::Value) {
    if let Some(obj) = value.as_object_mut() {
        obj.remove("_ferrite_agent_prompt_marker");
        obj.remove("_tauritavern_agent_prompt_marker");
    }
}

/// 400 + JSON body `{"error": "<msg>"}`，Content-Type application/json。
fn bad_request(msg: &str) -> Response {
    (StatusCode::BAD_REQUEST, Json(json!({"error": msg}))).into_response()
}

/// 把请求体当原始字节收下，不解析。转发必须保真。
struct BytesBody(bytes::Bytes);

impl<S> axum::extract::FromRequest<S> for BytesBody
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);
    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = axum::body::Bytes::from_request(req, state)
            .await
            .map_err(|_| (StatusCode::BAD_REQUEST, "body"))?;
        Ok(Self(bytes))
    }
}
