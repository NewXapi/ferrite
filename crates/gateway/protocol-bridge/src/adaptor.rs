//! `adaptor` —— 厂商协议适配器（对标 new-api `relay/channel/*/adaptor.go`）
//!
//! 每个厂商一个适配器，负责 **客户端协议 ↔ 厂商协议** 的双向转换（接口兼容）。
//! 原 protocol crate 的 Codec trait 迁入本模块（保持 `(source, target)` 有向
//! 字节转换语义），注册表从单协议查找扩展为组合路由查到 `(source, target)`。
//!
//! 组合语义 (借鉴 relaykit composed routes): Chat→Claude 直转是"直接路由",
//! Chat→Responses→Claude 是"组合路由"。TODO(#503): 组合路由二期, 先覆盖直接路由。

use bytes::Bytes;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// 协议族 — 值域对应 contract ChannelRecord.provider_type。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    OpenAi,
    Claude,
    Gemini,
    /// 其它厂商先透传 (零转换), 转换器按需增补。
    Passthrough,
}

/// 请求/响应格式转换器 — 一次尝试内的 (源格式 → 目标格式) 有向转换。
pub trait Codec: Send + Sync {
    fn source(&self) -> Protocol;
    fn target(&self) -> Protocol;
    /// 请求体转换 (一次性; 请求体通常较小)。
    fn adapt_request(&self, body: Bytes) -> Result<Bytes, AdaptorError>;
    /// 响应流转换 (逐块; chunk in → chunks out)。
    fn adapt_response(&self, chunk: Bytes) -> Result<Vec<Bytes>, AdaptorError>;
}

/// 适配器错误
#[derive(Debug, Error)]
pub enum AdaptorError {
    #[error("no adaptor for {from:?} -> {to:?}")]
    NotRegistered { from: Protocol, to: Protocol },
    #[error("encode failed: {0}")]
    EncodeFailed(String),
    #[error("decode failed: {0}")]
    DecodeFailed(String),
    #[error("unsupported conversion: {from:?} -> {to:?}")]
    Unsupported { from: Protocol, to: Protocol },
}

/// 适配器注册表 — `(source, target) → Codec` 查找。
/// forward 管道的唯一协议转换入口 — 业务代码不感知具体转换器实现。
pub struct AdaptorRegistry {
    codecs: HashMap<(Protocol, Protocol), Arc<dyn Codec>>,
}

impl AdaptorRegistry {
    pub fn new() -> Self {
        Self {
            codecs: HashMap::new(),
        }
    }

    /// 装载内置适配器 (openai 透传 + claude/gemini 转换骨架)。
    pub fn with_defaults() -> Self {
        let mut r = Self::new();
        r.register_openai();
        r.register_claude();
        r.register_gemini();
        r
    }

    fn register_openai(&mut self) {
        self.codecs
            .insert((Protocol::OpenAi, Protocol::OpenAi), Arc::new(OpenAiCodec));
    }

    fn register_claude(&mut self) {
        self.codecs.insert(
            (Protocol::OpenAi, Protocol::Claude),
            Arc::new(ClaudeCodec { to_claude: true }),
        );
        self.codecs.insert(
            (Protocol::Claude, Protocol::OpenAi),
            Arc::new(ClaudeCodec { to_claude: false }),
        );
    }

    fn register_gemini(&mut self) {
        self.codecs.insert(
            (Protocol::OpenAi, Protocol::Gemini),
            Arc::new(GeminiCodec { to_gemini: true }),
        );
        self.codecs.insert(
            (Protocol::Gemini, Protocol::OpenAi),
            Arc::new(GeminiCodec { to_gemini: false }),
        );
    }

    pub fn register(&mut self, codec: Arc<dyn Codec>) {
        self.codecs.insert((codec.source(), codec.target()), codec);
    }

    /// 查找 (source → target) 直接转换器; None = 不支持该组合。
    pub fn resolve(&self, source: Protocol, target: Protocol) -> Option<Arc<dyn Codec>> {
        if source == target {
            // 同格式透传兜底。
            return Some(Arc::new(PassthroughCodec));
        }
        self.codecs.get(&(source, target)).cloned()
    }
}

impl Default for AdaptorRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// 同格式透传 — 零转换。
pub struct PassthroughCodec;

impl Codec for PassthroughCodec {
    fn source(&self) -> Protocol {
        Protocol::Passthrough
    }
    fn target(&self) -> Protocol {
        Protocol::Passthrough
    }
    fn adapt_request(&self, body: Bytes) -> Result<Bytes, AdaptorError> {
        Ok(body)
    }
    fn adapt_response(&self, chunk: Bytes) -> Result<Vec<Bytes>, AdaptorError> {
        Ok(vec![chunk])
    }
}

/// OpenAI 兼容适配器 — 事实上的中心格式。
///
/// 设计立场: OpenAI Chat Completions 是**中枢格式**, X→OpenAI 与 OpenAI→Y
/// 各一个方向转换器, N 种协议互转从 O(N²) 降为 O(N)。new-api 的 channel
/// 适配器里 80% 厂商实际"路由到 openai adaptor"也验证了这点。
/// 转换点参考 new-api relay/channel/openai/adaptor.go。
pub struct OpenAiCodec;

impl Codec for OpenAiCodec {
    fn source(&self) -> Protocol {
        Protocol::OpenAi
    }
    fn target(&self) -> Protocol {
        Protocol::OpenAi
    }
    fn adapt_request(&self, body: Bytes) -> Result<Bytes, AdaptorError> {
        // 只有在需要注入 stream_options.include_usage 时才解析/重写
        // 否则零拷贝透传
        if body.is_empty() {
            return Ok(body);
        }

        let mut req: Value = match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(_) => return Ok(body), // 非 JSON 直接透传
        };

        let stream = req.get("stream").and_then(|v| v.as_bool()).unwrap_or(false);
        if stream {
            let mut stream_options = req
                .get_mut("stream_options")
                .and_then(|v| v.as_object_mut());
            let has_include_usage = stream_options
                .as_ref()
                .and_then(|o| o.get("include_usage"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if !has_include_usage {
                // 注入 stream_options.include_usage = true
                if stream_options.is_none() {
                    req["stream_options"] = json!({});
                    stream_options = req
                        .get_mut("stream_options")
                        .and_then(|v| v.as_object_mut());
                }
                if let Some(opts) = stream_options {
                    opts.insert("include_usage".to_string(), json!(true));
                }
                return Ok(Bytes::from(
                    serde_json::to_vec(&req)
                        .map_err(|e| AdaptorError::EncodeFailed(e.to_string()))?,
                ));
            }
        }
        Ok(body)
    }
    fn adapt_response(&self, chunk: Bytes) -> Result<Vec<Bytes>, AdaptorError> {
        Ok(vec![chunk])
    }
}

/// Claude (Anthropic Messages) 适配器。
/// 转换点参考 new-api relay/channel/claude_handler.go + relaykit 内建。
/// 请求: system 顶层化 / messages 重排 / tool 块映射 / max_tokens 必填默认;
/// 响应: message_start/content_block_delta/message_stop → OpenAI chunk 流。
pub struct ClaudeCodec {
    /// true: 目标是 Claude (入参为 OpenAI 格式); false: 反向。
    pub to_claude: bool,
}

impl Codec for ClaudeCodec {
    fn source(&self) -> Protocol {
        if self.to_claude {
            Protocol::OpenAi
        } else {
            Protocol::Claude
        }
    }
    fn target(&self) -> Protocol {
        if self.to_claude {
            Protocol::Claude
        } else {
            Protocol::OpenAi
        }
    }
    fn adapt_request(&self, body: Bytes) -> Result<Bytes, AdaptorError> {
        if !self.to_claude {
            return Err(AdaptorError::Unsupported {
                from: Protocol::Claude,
                to: Protocol::OpenAi,
            });
        }
        if body.is_empty() {
            return Ok(body);
        }

        let req: Value =
            serde_json::from_slice(&body).map_err(|e| AdaptorError::DecodeFailed(e.to_string()))?;

        // Extract system messages
        let mut system_content = Vec::new();
        let mut claude_messages = Vec::new();

        if let Some(messages) = req.get("messages").and_then(|v| v.as_array()) {
            for msg in messages {
                let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("user");
                let content = msg.get("content").cloned();

                if role == "system" {
                    // Extract system message to top-level system field
                    if let Some(text) = extract_text_content(&content) {
                        if !text.is_empty() {
                            system_content.push(json!({
                                "type": "text",
                                "text": text
                            }));
                        }
                    }
                    continue;
                }

                let mut claude_msg = json!({
                    "role": role,
                });

                if let Some(c) = content {
                    if role == "tool" {
                        // Tool result message
                        let tool_call_id = msg
                            .get("tool_call_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        claude_msg["content"] = json!([{
                            "type": "tool_result",
                            "tool_use_id": tool_call_id,
                            "content": c
                        }]);
                    } else if let Some(text) = extract_text_content(&Some(c.clone())) {
                        // Simple text content
                        if let Some(tool_calls) = msg.get("tool_calls").and_then(|v| v.as_array()) {
                            // Assistant with tool calls
                            let mut blocks = vec![json!({
                                "type": "text",
                                "text": text
                            })];
                            for tc in tool_calls {
                                let name = tc
                                    .get("function")
                                    .and_then(|f| f.get("name"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let args = tc
                                    .get("function")
                                    .and_then(|f| f.get("arguments"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("{}");
                                let id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                blocks.push(json!({
                                    "type": "tool_use",
                                    "id": id,
                                    "name": name,
                                    "input": serde_json::from_str::<Value>(args).unwrap_or(json!({}))
                                }));
                            }
                            claude_msg["content"] = json!(blocks);
                        } else {
                            // Simple text message
                            claude_msg["content"] = json!(Value::String(text));
                        }
                    } else if let Some(arr) = c.as_array() {
                        // Content array (text + images)
                        let mut blocks = Vec::new();
                        for item in arr {
                            let item_type =
                                item.get("type").and_then(|v| v.as_str()).unwrap_or("text");
                            match item_type {
                                "text" => {
                                    if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                                        if !text.is_empty() {
                                            blocks.push(json!({
                                                "type": "text",
                                                "text": text
                                            }));
                                        }
                                    }
                                }
                                "image_url" => {
                                    if let Some(image_url) =
                                        item.get("image_url").and_then(|v| v.as_object())
                                    {
                                        if let Some(url) =
                                            image_url.get("url").and_then(|v| v.as_str())
                                        {
                                            // Extract base64 data from data URL or assume it's base64
                                            blocks.push(json!({
                                                "type": "image",
                                                "source": {
                                                    "type": "base64",
                                                    "media_type": "image/png",
                                                    "data": url
                                                }
                                            }));
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        if !blocks.is_empty() {
                            claude_msg["content"] = json!(blocks);
                        }
                    }
                }

                claude_messages.push(claude_msg);
            }
        }

        // Build Claude request
        let mut claude_req = json!({
            "model": req.get("model").and_then(|v| v.as_str()).unwrap_or(""),
            "messages": claude_messages,
        });

        // Add system if present
        if !system_content.is_empty() {
            claude_req["system"] = json!(system_content);
        }

        // max_tokens: required, default 4096
        let max_tokens = req.get("max_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let default_max_tokens = get_default_max_tokens(req.get("model").and_then(|v| v.as_str()));
        if max_tokens > 0 {
            claude_req["max_tokens"] = json!(max_tokens);
        } else if default_max_tokens > 0 {
            claude_req["max_tokens"] = json!(default_max_tokens);
        }

        // Optional parameters
        if let Some(temp) = req.get("temperature").and_then(|v| v.as_f64()) {
            claude_req["temperature"] = json!(temp);
        }
        if let Some(top_p) = req.get("top_p").and_then(|v| v.as_f64()) {
            claude_req["top_p"] = json!(top_p);
        }
        if let Some(top_k) = req.get("top_k").and_then(|v| v.as_u64()) {
            claude_req["top_k"] = json!(top_k);
        }
        if let Some(stop) = req.get("stop") {
            if let Some(s) = stop.as_str() {
                claude_req["stop_sequences"] = json!([s]);
            } else if let Some(arr) = stop.as_array() {
                let mut seqs = Vec::new();
                for item in arr {
                    if let Some(s) = item.as_str() {
                        seqs.push(s);
                    }
                }
                if !seqs.is_empty() {
                    claude_req["stop_sequences"] = json!(seqs);
                }
            }
        }
        if let Some(tools) = req.get("tools").and_then(|v| v.as_array()) {
            let mut claude_tools = Vec::new();
            for tool in tools {
                if let Some(func) = tool.get("function").and_then(|v| v.as_object()) {
                    let name = func.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let desc = func
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let params = func.get("parameters").cloned().unwrap_or(json!({}));
                    claude_tools.push(json!({
                        "name": name,
                        "description": desc,
                        "input_schema": params
                    }));
                }
            }
            if !claude_tools.is_empty() {
                claude_req["tools"] = json!(claude_tools);
            }
        }
        if let Some(tool_choice) = req.get("tool_choice") {
            claude_req["tool_choice"] = tool_choice.clone();
        }
        if let Some(stream) = req.get("stream").and_then(|v| v.as_bool()) {
            claude_req["stream"] = json!(stream);
        }

        Ok(Bytes::from(
            serde_json::to_vec(&claude_req)
                .map_err(|e| AdaptorError::EncodeFailed(e.to_string()))?,
        ))
    }
    fn adapt_response(&self, chunk: Bytes) -> Result<Vec<Bytes>, AdaptorError> {
        if self.to_claude {
            return Err(AdaptorError::Unsupported {
                from: Protocol::OpenAi,
                to: Protocol::Claude,
            });
        }
        if chunk.is_empty() {
            return Ok(vec![]);
        }

        let chunk_str = String::from_utf8_lossy(&chunk);
        let is_sse = chunk_str
            .lines()
            .any(|line| line.trim().starts_with("data: "));

        if is_sse {
            let mut results = Vec::new();
            for line in chunk_str.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with(':') {
                    continue;
                }
                if line == "data: [DONE]" {
                    results.push(Bytes::from("data: [DONE]\n\n"));
                    continue;
                }
                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(claude_event) = serde_json::from_str::<Value>(data) {
                        if let Some(openai_chunks) = convert_claude_event_to_openai(&claude_event) {
                            for oc in openai_chunks {
                                results.push(Bytes::from(format!("data: {}\n\n", oc)));
                            }
                        }
                    }
                }
            }
            Ok(results)
        } else {
            let claude_resp: Value = serde_json::from_str(&chunk_str)
                .map_err(|e| AdaptorError::DecodeFailed(e.to_string()))?;

            if let Some(content_val) = claude_resp.get("content") {
                let content_text = extract_text_from_claude_content(Some(content_val));
                let mut openai_resp = json!({
                    "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                    "object": "chat.completion",
                    "created": chrono::Utc::now().timestamp(),
                    "model": claude_resp.get("model").and_then(|v| v.as_str()).unwrap_or(""),
                    "choices": [{
                        "index": 0,
                        "message": {
                            "role": claude_resp.get("role").and_then(|v| v.as_str()).unwrap_or("assistant"),
                            "content": content_text.unwrap_or_else(|| "".to_string()),
                        },
                        "finish_reason": claude_resp.get("stop_reason").and_then(|v| v.as_str()).unwrap_or("stop")
                    }],
                    "usage": {
                        "prompt_tokens": claude_resp.get("usage").and_then(|u| u.get("input_tokens")).and_then(|v| v.as_u64()).unwrap_or(0),
                        "completion_tokens": claude_resp.get("usage").and_then(|u| u.get("output_tokens")).and_then(|v| v.as_u64()).unwrap_or(0),
                        "total_tokens": claude_resp.get("usage").and_then(|u| u.get("input_tokens")).and_then(|v| v.as_u64()).unwrap_or(0) + claude_resp.get("usage").and_then(|u| u.get("output_tokens")).and_then(|v| v.as_u64()).unwrap_or(0)
                    }
                });
                Ok(vec![Bytes::from(serde_json::to_vec(&openai_resp).unwrap())])
            } else if let Some(event_type) = claude_resp.get("type").and_then(|v| v.as_str()) {
                if let Some(openai_chunks) = convert_claude_event_to_openai(&claude_resp) {
                    let mut results = Vec::new();
                    for oc in openai_chunks {
                        results.push(Bytes::from(format!("data: {}\n\n", oc)));
                    }
                    Ok(results)
                } else {
                    Ok(vec![])
                }
            } else {
                Ok(vec![])
            }
        }
    }
}

/// Gemini 适配器。
/// 转换点参考 new-api relay/channel/gemini_handler.go + relaykit 内建。
/// 请求: contents/role 映射, systemInstruction 独立化;
/// 响应: candidates/usageMetadata → OpenAI chunk。
pub struct GeminiCodec {
    /// true: 目标是 Gemini。
    pub to_gemini: bool,
}

impl Codec for GeminiCodec {
    fn source(&self) -> Protocol {
        if self.to_gemini {
            Protocol::OpenAi
        } else {
            Protocol::Gemini
        }
    }
    fn target(&self) -> Protocol {
        if self.to_gemini {
            Protocol::Gemini
        } else {
            Protocol::OpenAi
        }
    }
    fn adapt_request(&self, body: Bytes) -> Result<Bytes, AdaptorError> {
        if !self.to_gemini {
            return Err(AdaptorError::Unsupported {
                from: Protocol::Gemini,
                to: Protocol::OpenAi,
            });
        }
        if body.is_empty() {
            return Ok(body);
        }

        let req: Value =
            serde_json::from_slice(&body).map_err(|e| AdaptorError::DecodeFailed(e.to_string()))?;

        let mut contents = Vec::new();
        let mut system_instruction = None;

        if let Some(messages) = req.get("messages").and_then(|v| v.as_array()) {
            for msg in messages {
                let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("user");
                let content = msg.get("content").cloned();

                if role == "system" {
                    if let Some(text) = extract_text_content(&content) {
                        if !text.is_empty() {
                            system_instruction = Some(json!({
                                "parts": [{ "text": text }]
                            }));
                        }
                    }
                    continue;
                }

                let gemini_role = if role == "assistant" { "model" } else { "user" };
                let mut parts = Vec::new();

                if let Some(text) = extract_text_content(&content) {
                    if !text.is_empty() {
                        parts.push(json!({ "text": text }));
                    }
                } else if let Some(content_val) = content.as_ref() {
                    if let Some(arr) = content_val.as_array() {
                        for item in arr {
                            let item_type =
                                item.get("type").and_then(|v| v.as_str()).unwrap_or("text");
                            match item_type {
                                "text" => {
                                    if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                                        if !text.is_empty() {
                                            parts.push(json!({ "text": text }));
                                        }
                                    }
                                }
                                "image_url" => {
                                    if let Some(image_url) =
                                        item.get("image_url").and_then(|v| v.as_object())
                                    {
                                        if let Some(url) =
                                            image_url.get("url").and_then(|v| v.as_str())
                                        {
                                            let data = if url.starts_with("data:") {
                                                url.split(',').nth(1).unwrap_or("")
                                            } else {
                                                url
                                            };
                                            parts.push(json!({
                                                "inline_data": {
                                                    "mime_type": "image/png",
                                                    "data": data
                                                }
                                            }));
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }

                if !parts.is_empty() {
                    contents.push(json!({
                        "role": gemini_role,
                        "parts": parts
                    }));
                }
            }
        }

        let mut gemini_req = json!({
            "contents": contents,
        });

        if let Some(sys) = system_instruction {
            gemini_req["system_instruction"] = sys;
        }

        let mut gen_config = json!({});
        if let Some(temp) = req.get("temperature").and_then(|v| v.as_f64()) {
            gen_config["temperature"] = json!(temp);
        }
        if let Some(top_p) = req.get("top_p").and_then(|v| v.as_f64()) {
            gen_config["top_p"] = json!(top_p);
        }
        if let Some(max_tokens) = req.get("max_tokens").and_then(|v| v.as_u64()) {
            gen_config["max_output_tokens"] = json!(max_tokens);
        }
        if let Some(stop) = req.get("stop") {
            if let Some(s) = stop.as_str() {
                gen_config["stop_sequences"] = json!([s]);
            } else if let Some(arr) = stop.as_array() {
                let mut seqs = Vec::new();
                for item in arr {
                    if let Some(s) = item.as_str() {
                        seqs.push(s);
                    }
                }
                if !seqs.is_empty() {
                    gen_config["stop_sequences"] = json!(seqs);
                }
            }
        }
        if let Some(stream) = req.get("stream").and_then(|v| v.as_bool()) {
            gemini_req["generation_config"] = gen_config;
            gemini_req["stream"] = json!(stream);
        } else if gen_config.as_object().map_or(false, |o| !o.is_empty()) {
            gemini_req["generation_config"] = gen_config;
        }

        Ok(Bytes::from(
            serde_json::to_vec(&gemini_req)
                .map_err(|e| AdaptorError::EncodeFailed(e.to_string()))?,
        ))
    }
    fn adapt_response(&self, chunk: Bytes) -> Result<Vec<Bytes>, AdaptorError> {
        if self.to_gemini {
            return Err(AdaptorError::Unsupported {
                from: Protocol::OpenAi,
                to: Protocol::Gemini,
            });
        }
        if chunk.is_empty() {
            return Ok(vec![]);
        }

        let chunk_str = String::from_utf8_lossy(&chunk);
        let mut results = Vec::new();

        for line in chunk_str.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(':') {
                continue;
            }
            if line == "data: [DONE]" {
                results.push(Bytes::from("data: [DONE]\n\n"));
                continue;
            }
            if let Some(data) = line.strip_prefix("data: ") {
                if let Ok(gemini_event) = serde_json::from_str::<Value>(data) {
                    if let Some(openai_chunks) = convert_gemini_event_to_openai(&gemini_event) {
                        for oc in openai_chunks {
                            results.push(Bytes::from(format!("data: {}\n\n", oc)));
                        }
                    }
                }
            }
        }

        if results.is_empty() {
            // Handle non-SSE format (raw JSON)
            if let Ok(gemini_event) = serde_json::from_str::<Value>(&chunk_str) {
                if let Some(openai_chunks) = convert_gemini_event_to_openai(&gemini_event) {
                    // Check if this is a complete response (has usage_metadata or finish_reason)
                    let is_complete_response = gemini_event.get("usage_metadata").is_some()
                        || gemini_event
                            .get("candidates")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().any(|c| c.get("finish_reason").is_some()))
                            .unwrap_or(false);

                    if is_complete_response {
                        // Return as raw JSON (OpenAI chat completion format)
                        // Combine all chunks into a single response
                        let mut combined = json!({
                            "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                            "object": "chat.completion",
                            "created": chrono::Utc::now().timestamp(),
                            "model": gemini_event.get("model").and_then(|v| v.as_str()).unwrap_or(""),
                            "choices": [],
                            "usage": {
                                "prompt_tokens": 0,
                                "completion_tokens": 0,
                                "total_tokens": 0
                            }
                        });

                        for oc_str in openai_chunks {
                            if let Ok(chunk) = serde_json::from_str::<Value>(&oc_str) {
                                if let Some(usage) = chunk.get("usage") {
                                    combined["usage"] = usage.clone();
                                } else if let Some(choices) =
                                    chunk.get("choices").and_then(|v| v.as_array())
                                {
                                    combined["choices"] = Value::Array(choices.clone());
                                }
                            }
                        }
                        results.push(Bytes::from(serde_json::to_vec(&combined).unwrap()));
                    } else {
                        // Return as SSE chunks
                        for oc in openai_chunks {
                            results.push(Bytes::from(format!("data: {}\n\n", oc)));
                        }
                    }
                } else {
                    return Err(AdaptorError::DecodeFailed(chunk_str.to_string()));
                }
            } else {
                return Err(AdaptorError::DecodeFailed(chunk_str.to_string()));
            }
        }

        Ok(results)
    }
}

fn extract_text_content(content: &Option<Value>) -> Option<String> {
    content.as_ref().and_then(|c| {
        if let Some(text) = c.as_str() {
            Some(text.to_string())
        } else if let Some(arr) = c.as_array() {
            let mut texts = Vec::new();
            for item in arr {
                if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                    texts.push(t);
                }
            }
            if texts.is_empty() {
                None
            } else {
                Some(texts.join(" "))
            }
        } else {
            None
        }
    })
}

fn extract_text_from_claude_content(content: Option<&Value>) -> Option<String> {
    content.and_then(|c| {
        if let Some(text) = c.as_str() {
            Some(text.to_string())
        } else if let Some(arr) = c.as_array() {
            let mut texts = Vec::new();
            for item in arr {
                if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                    texts.push(t);
                }
            }
            if texts.is_empty() {
                None
            } else {
                Some(texts.join(" "))
            }
        } else {
            None
        }
    })
}

fn get_default_max_tokens(model: Option<&str>) -> u64 {
    match model {
        Some(m) if m.contains("opus") => 4096,
        Some(m) if m.contains("sonnet") => 4096,
        Some(m) if m.contains("haiku") => 4096,
        _ => 4096,
    }
}

fn convert_claude_event_to_openai(event: &Value) -> Option<Vec<String>> {
    let event_type = event.get("type").and_then(|v| v.as_str())?;

    match event_type {
        "message_start" => {
            if let Some(message) = event.get("message") {
                let role = message
                    .get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("assistant");
                let content = message.get("content").and_then(|v| v.as_array());
                let mut openai_chunk = json!({
                    "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                    "object": "chat.completion.chunk",
                    "created": chrono::Utc::now().timestamp(),
                    "model": message.get("model").and_then(|v| v.as_str()).unwrap_or(""),
                    "choices": [{
                        "index": 0,
                        "delta": {
                            "role": role,
                            "content": ""
                        },
                        "finish_reason": null
                    }]
                });
                if let Some(arr) = content {
                    if let Some(first) = arr.first() {
                        if let Some(text) = first.get("text").and_then(|v| v.as_str()) {
                            if !text.is_empty() {
                                openai_chunk["choices"][0]["delta"]["content"] = json!(text);
                            }
                        }
                    }
                }
                return Some(vec![serde_json::to_string(&openai_chunk).ok()?]);
            }
        }
        "content_block_delta" => {
            if let Some(delta) = event.get("delta") {
                let delta_type = delta.get("type").and_then(|v| v.as_str());
                let mut openai_chunk = json!({
                    "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                    "object": "chat.completion.chunk",
                    "created": chrono::Utc::now().timestamp(),
                    "model": event.get("model").and_then(|v| v.as_str()).unwrap_or(""),
                    "choices": [{
                        "index": 0,
                        "delta": {},
                        "finish_reason": null
                    }]
                });
                match delta_type {
                    Some("text_delta") => {
                        if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                            openai_chunk["choices"][0]["delta"]["content"] = json!(text);
                        }
                    }
                    Some("input_json_delta") => {
                        if let Some(partial_json) =
                            delta.get("partial_json").and_then(|v| v.as_str())
                        {
                            openai_chunk["choices"][0]["delta"]["tool_calls"] = json!([{
                                "index": 0,
                                "type": "function",
                                "function": {
                                    "arguments": partial_json
                                }
                            }]);
                        }
                    }
                    _ => {}
                }
                return Some(vec![serde_json::to_string(&openai_chunk).ok()?]);
            }
        }
        "message_delta" => {
            if let Some(delta) = event.get("delta") {
                if let Some(stop_reason) = delta.get("stop_reason").and_then(|v| v.as_str()) {
                    let mut openai_chunk = json!({
                        "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                        "object": "chat.completion.chunk",
                        "created": chrono::Utc::now().timestamp(),
                        "model": event.get("model").and_then(|v| v.as_str()).unwrap_or(""),
                        "choices": [{
                            "index": 0,
                            "delta": {},
                            "finish_reason": stop_reason
                        }]
                    });
                    return Some(vec![serde_json::to_string(&openai_chunk).ok()?]);
                }
            }
            if let Some(usage) = event.get("usage") {
                let mut openai_chunk = json!({
                    "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                    "object": "chat.completion.chunk",
                    "created": chrono::Utc::now().timestamp(),
                    "model": event.get("model").and_then(|v| v.as_str()).unwrap_or(""),
                    "choices": [],
                    "usage": {
                        "prompt_tokens": usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                        "completion_tokens": usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                        "total_tokens": usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) + usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0)
                    }
                });
                return Some(vec![serde_json::to_string(&openai_chunk).ok()?]);
            }
        }
        "message_stop" => {
            let mut openai_chunk = json!({
                "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                "object": "chat.completion.chunk",
                "created": chrono::Utc::now().timestamp(),
                "model": event.get("model").and_then(|v| v.as_str()).unwrap_or(""),
                "choices": [{
                    "index": 0,
                    "delta": {},
                    "finish_reason": "stop"
                }]
            });
            return Some(vec![serde_json::to_string(&openai_chunk).ok()?]);
        }
        "error" => {
            let mut openai_chunk = json!({
                "error": event.get("error")
            });
            return Some(vec![serde_json::to_string(&openai_chunk).ok()?]);
        }
        _ => {}
    }
    None
}

fn convert_gemini_event_to_openai(event: &Value) -> Option<Vec<String>> {
    if let Some(candidates) = event.get("candidates").and_then(|v| v.as_array()) {
        let mut openai_chunks = Vec::new();
        for (idx, candidate) in candidates.iter().enumerate() {
            let mut openai_chunk = json!({
                "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                "object": "chat.completion.chunk",
                "created": chrono::Utc::now().timestamp(),
                "model": event.get("model").and_then(|v| v.as_str()).unwrap_or(""),
                "choices": [{
                    "index": idx,
                    "delta": {},
                    "finish_reason": null
                }]
            });

            if let Some(content) = candidate.get("content").and_then(|v| v.as_object()) {
                if let Some(parts) = content.get("parts").and_then(|v| v.as_array()) {
                    for part in parts {
                        if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                            openai_chunk["choices"][0]["delta"]["content"] = json!(text);
                        }
                    }
                }
            }

            if let Some(finish_reason) = candidate.get("finish_reason").and_then(|v| v.as_str()) {
                openai_chunk["choices"][0]["finish_reason"] = json!(finish_reason);
            }

            openai_chunks.push(serde_json::to_string(&openai_chunk).ok()?);
        }

        if let Some(usage) = event.get("usage_metadata") {
            let mut usage_chunk = json!({
                "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                "object": "chat.completion.chunk",
                "created": chrono::Utc::now().timestamp(),
                "model": event.get("model").and_then(|v| v.as_str()).unwrap_or(""),
                "choices": [],
                "usage": {
                    "prompt_tokens": usage.get("prompt_token_count").and_then(|v| v.as_u64()).unwrap_or(0),
                    "completion_tokens": usage.get("candidates_token_count").and_then(|v| v.as_u64()).unwrap_or(0),
                    "total_tokens": usage.get("total_token_count").and_then(|v| v.as_u64()).unwrap_or(0)
                }
            });
            openai_chunks.push(serde_json::to_string(&usage_chunk).ok()?);
        }

        if openai_chunks.is_empty() {
            None
        } else {
            Some(openai_chunks)
        }
    } else {
        None
    }
}

impl std::fmt::Debug for dyn Codec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Codec")
            .field("source", &self.source())
            .field("target", &self.target())
            .finish()
    }
}
