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
use futures_util::StreamExt;
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
        .body(body.0)
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
    if let Some(ct) = resp.headers().get(reqwest::header::CONTENT_TYPE) {
        if let Ok(v) = ct.to_str() {
            out = out.header("content-type", v);
        }
    }
    let stream = resp
        .bytes_stream()
        .map(|r| r.map_err(|e| std::io::Error::other(e)));
    out.body(Body::from_stream(stream))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
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
