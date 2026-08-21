//! 协议适配 + 上游转发

use bytes::Bytes;
use thiserror::Error;

/// 上游响应（非流式）
#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub body: Bytes,
    pub content_type: Option<String>,
}

/// 上游流式响应
#[derive(Debug)]
pub struct StreamResponse {
    pub status: u16,
    pub stream: reqwest::Response,
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
                // 不设整体 timeout（会截断长 SSE 流）：connect 限建连，read 限单次读空闲
                .connect_timeout(std::time::Duration::from_secs(10))
                .read_timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("failed to build reqwest client"),
        }
    }

    /// 转发 chat/completions 请求到上游（非流式）
    pub async fn forward(
        &self,
        body: Bytes,
        base_url: &str,
        api_key: &str,
        upstream_model: &str,
    ) -> Result<Response, AdapterError> {
        let json = self.prepare_body(body, upstream_model)?;
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

        let resp = self.send_request(&url, api_key, &json).await?;

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

    /// 转发 chat/completions 请求到上游（流式 SSE）
    ///
    /// 返回 reqwest::Response，调用方从 .bytes_stream() 读取 SSE chunks
    pub async fn forward_stream(
        &self,
        body: Bytes,
        base_url: &str,
        api_key: &str,
        upstream_model: &str,
    ) -> Result<StreamResponse, AdapterError> {
        let json = self.prepare_body(body, upstream_model)?;
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

        let resp = self.send_request(&url, api_key, &json).await?;

        let status = resp.status().as_u16();
        tracing::info!(status, "upstream stream response");
        Ok(StreamResponse {
            status,
            stream: resp,
        })
    }

    fn prepare_body(
        &self,
        body: Bytes,
        upstream_model: &str,
    ) -> Result<serde_json::Value, AdapterError> {
        let mut json: serde_json::Value = serde_json::from_slice(&body)
            .map_err(|e| AdapterError::Upstream(format!("invalid request body: {e}")))?;

        if let Some(obj) = json.as_object_mut() {
            obj.insert(
                "model".into(),
                serde_json::Value::String(upstream_model.to_string()),
            );
        }

        Ok(json)
    }

    async fn send_request(
        &self,
        url: &str,
        api_key: &str,
        json: &serde_json::Value,
    ) -> Result<reqwest::Response, AdapterError> {
        Ok(self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(json)
            .send()
            .await?)
    }
}

impl Default for OpenAIAdapter {
    fn default() -> Self {
        Self::new()
    }
}
