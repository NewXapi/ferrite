//! 协议适配 + 上游转发

use bytes::Bytes;
use thiserror::Error;

/// 上游响应
#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub body: Bytes,
    pub content_type: Option<String>,
}

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("upstream error: {0}")]
    Upstream(String),
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
}

/// OpenAI 格式适配器
pub struct OpenAIAdapter {
    client: reqwest::Client,
}

impl OpenAIAdapter {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("failed to build reqwest client"),
        }
    }

    /// 转发 chat/completions 请求到上游
    ///
    /// - `body`: 原始请求 body (透传)
    /// - `base_url`: 上游 base URL (如 `http://localhost:3100/v1`)
    /// - `api_key`: 上游 API key
    /// - `upstream_model`: 替换 model 字段后的上游模型名
    pub async fn forward(
        &self,
        body: Bytes,
        base_url: &str,
        api_key: &str,
        upstream_model: &str,
    ) -> Result<Response, AdapterError> {
        let mut json: serde_json::Value = serde_json::from_slice(&body)
            .map_err(|e| AdapterError::Upstream(format!("invalid request body: {e}")))?;

        if let Some(obj) = json.as_object_mut() {
            obj.insert(
                "model".into(),
                serde_json::Value::String(upstream_model.to_string()),
            );
        }

        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&json)
            .send()
            .await?;

        let status = resp.status().as_u16();
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let body = resp.bytes().await?;

        tracing::info!(status, body_len = body.len(), "upstream response");
        Ok(Response {
            status,
            body,
            content_type,
        })
    }
}

impl Default for OpenAIAdapter {
    fn default() -> Self {
        Self::new()
    }
}
