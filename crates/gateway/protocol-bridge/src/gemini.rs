//! Gemini 格式 codec —— IR ↔ `generateContent`（WP4）。
//!
//! ## 与 OpenAI/Claude 的形状差异
//!
//! - 消息叫 `contents`，角色只有 `user` / `model`（没有 system / assistant / tool）。
//! - 系统提示独立成 `systemInstruction`，不占 contents 位。
//! - 生成参数全在 `generationConfig` 里（`maxOutputTokens` / `topP` / `stopSequences`）。
//! - 工具声明用 `functionDeclarations`，其 schema 字段叫 `parameters`（不是 Chat 的
//!   `parameters` 包在 function 里，也不是 IR 的 `input_schema`）。
//! - 工具调用是 `functionCall` **整体对象**，没有 id；流式也不像 OpenAI 那样把参数
//!   切片下发，而是（要么整体给、要么重复给累积后的对象）。
//!
//! ## 本模块修掉的三个既有缺陷（G1）
//!
//! 旧实现 `adaptor::convert_gemini_event_to_openai` 里有三处糙点：
//!
//! 1. **`finish_reason` 原样透传**——Gemini 发的是 `STOP` / `MAX_TOKENS` / `SAFETY`
//!    这类大写枚举，直接塞进 OpenAI 形状的 `finish_reason` 会让客户端认不出来。
//!    现在经 [`gemini_finish_reason`] 映射。
//! 2. **chunk id 每块现生成 uuid**——同一次补全的多个 chunk 各自带不同 id，客户端
//!    与流式计费聚合都会把它当成多次响应。现在由 [`GeminiStreamEncoder`] 全程复用。
//! 3. **usage 块的 `choices` 是空数组**——多数客户端会拒绝这种形状。现在 usage 帧
//!    带非空 choices。

use bytes::Bytes;
use serde_json::{Map, Value, json};

use crate::adaptor::{AdaptorError, Protocol};
use crate::format_codec::{FormatCodec, StreamEncoder};
use crate::ir::{
    ContentBlock, LlmRequest, LlmResponse, Message, Role, SamplingParams, StopReason, StreamEvent,
    TextBlock, ToolDef, Usage,
};

/// Gemini 格式 codec。
#[derive(Debug, Default, Clone, Copy)]
pub struct GeminiCodec;

impl GeminiCodec {
    /// 新建 codec（无状态，可共享）。
    pub const fn new() -> Self {
        Self
    }
}

impl FormatCodec for GeminiCodec {
    fn format(&self) -> Protocol {
        Protocol::Gemini
    }

    /// Gemini 请求体 → IR。
    fn decode_request(&self, body: Bytes) -> Result<LlmRequest, AdaptorError> {
        let v: Value =
            serde_json::from_slice(&body).map_err(|e| AdaptorError::DecodeFailed(e.to_string()))?;
        let obj = v.as_object().ok_or_else(|| {
            AdaptorError::DecodeFailed("gemini request body is not an object".into())
        })?;

        let system = obj
            .get("systemInstruction")
            .and_then(|s| s.get("parts"))
            .and_then(Value::as_array)
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(|p| {
                        p.get("text").and_then(Value::as_str).map(|t| TextBlock {
                            text: t.to_string(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut messages = Vec::new();
        if let Some(contents) = obj.get("contents").and_then(Value::as_array) {
            for c in contents {
                // Gemini 只有 user / model 两个角色；model 即 assistant。
                let role = match c.get("role").and_then(Value::as_str) {
                    Some("model") => Role::Assistant,
                    _ => Role::User,
                };
                let content = c
                    .get("parts")
                    .and_then(Value::as_array)
                    .map(|parts| parts.iter().map(decode_part).collect::<Vec<_>>())
                    .unwrap_or_default();
                messages.push(Message { role, content });
            }
        }

        // functionDeclarations 可以出现在多个 tools 项里，要摊平。
        let mut tools = Vec::new();
        if let Some(arr) = obj.get("tools").and_then(Value::as_array) {
            for t in arr {
                let Some(decls) = t.get("functionDeclarations").and_then(Value::as_array) else {
                    continue;
                };
                for d in decls {
                    let Some(name) = d.get("name").and_then(Value::as_str) else {
                        continue;
                    };
                    tools.push(ToolDef {
                        name: name.to_string(),
                        description: d
                            .get("description")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        // Gemini 用 `parameters`；IR 侧统一叫 input_schema。
                        input_schema: d.get("parameters").cloned().unwrap_or(json!({})),
                        extra: Map::new(),
                    });
                }
            }
        }

        let generation_config = obj
            .get("generationConfig")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let temperature = generation_config.get("temperature").and_then(Value::as_f64);
        let top_p = generation_config.get("topP").and_then(Value::as_f64);
        let max_tokens = generation_config
            .get("maxOutputTokens")
            .and_then(Value::as_u64);
        let mut sampling_extra = Map::new();
        if let Some(stop) = generation_config.get("stopSequences") {
            sampling_extra.insert("stop".into(), stop.clone());
        }
        let sampling = if temperature.is_none()
            && top_p.is_none()
            && max_tokens.is_none()
            && sampling_extra.is_empty()
        {
            None
        } else {
            Some(SamplingParams {
                temperature,
                top_p,
                max_tokens,
                extra: sampling_extra,
            })
        };

        let mut extra = Map::new();
        for (k, val) in obj {
            if !matches!(
                k.as_str(),
                "contents" | "systemInstruction" | "tools" | "generationConfig" | "stream"
            ) {
                extra.insert(k.clone(), val.clone());
            }
        }

        Ok(LlmRequest {
            model: String::new(), // Gemini 的模型标识在 URL 路径里，不在请求体。
            messages,
            system,
            tools,
            tool_choice: None,
            sampling,
            stream: obj.get("stream").and_then(Value::as_bool).unwrap_or(false),
            extra,
        })
    }

    /// IR → Gemini 请求体。
    fn encode_request(&self, req: &LlmRequest) -> Result<Bytes, AdaptorError> {
        let mut out = Map::new();

        if !req.system.is_empty() {
            out.insert(
                "systemInstruction".into(),
                json!({
                    "parts": req.system.iter().map(|b| json!({"text": b.text})).collect::<Vec<_>>(),
                }),
            );
        }

        let contents: Vec<Value> = req.messages.iter().map(encode_content).collect();
        out.insert("contents".into(), Value::Array(contents));

        if !req.tools.is_empty() {
            let decls: Vec<Value> = req
                .tools
                .iter()
                .map(|t| {
                    let mut d = json!({
                        "name": t.name,
                        "parameters": t.input_schema,
                    });
                    if let Some(desc) = &t.description {
                        d["description"] = json!(desc);
                    }
                    d
                })
                .collect();
            out.insert("tools".into(), json!([{"functionDeclarations": decls}]));
        }

        if let Some(s) = &req.sampling {
            let mut cfg = Map::new();
            if let Some(t) = s.temperature {
                cfg.insert("temperature".into(), json!(t));
            }
            if let Some(p) = s.top_p {
                cfg.insert("topP".into(), json!(p));
            }
            if let Some(m) = s.max_tokens {
                cfg.insert("maxOutputTokens".into(), json!(m));
            }
            if let Some(stop) = s.extra.get("stop") {
                // Gemini 用 stopSequences，且必须是数组。
                let seqs = match stop {
                    Value::String(text) => json!([text]),
                    other => other.clone(),
                };
                cfg.insert("stopSequences".into(), seqs);
            }
            if !cfg.is_empty() {
                out.insert("generationConfig".into(), Value::Object(cfg));
            }
        }

        out.insert("stream".into(), json!(req.stream));

        for (k, v) in &req.extra {
            out.entry(k.clone()).or_insert_with(|| v.clone());
        }

        serde_json::to_vec(&Value::Object(out))
            .map(Bytes::from)
            .map_err(|e| AdaptorError::EncodeFailed(e.to_string()))
    }

    /// Gemini 非流式响应体 → IR。
    fn decode_response(&self, body: Bytes) -> Result<LlmResponse, AdaptorError> {
        let v: Value =
            serde_json::from_slice(&body).map_err(|e| AdaptorError::DecodeFailed(e.to_string()))?;

        let candidate = v
            .get("candidates")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .cloned()
            .unwrap_or(json!({}));

        let outputs = candidate
            .get("content")
            .and_then(|c| c.get("parts"))
            .and_then(Value::as_array)
            .map(|parts| parts.iter().map(decode_part).collect::<Vec<_>>())
            .unwrap_or_default();

        let stop_reason = candidate
            .get("finishReason")
            .and_then(Value::as_str)
            .map(gemini_finish_reason)
            // 没有 finishReason 时，有工具调用就按工具调用收尾。
            .unwrap_or_else(|| {
                if outputs
                    .iter()
                    .any(|b| matches!(b, ContentBlock::ToolUse { .. }))
                {
                    StopReason::ToolUse
                } else {
                    StopReason::Stop
                }
            });

        Ok(LlmResponse {
            // Gemini 不返回响应 id；用 usage 之外无稳定标识，故留空由上层补。
            id: v
                .get("responseId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            model: v
                .get("modelVersion")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            outputs,
            usage: decode_usage(v.get("usageMetadata")),
            stop_reason,
        })
    }

    /// IR → Gemini 非流式响应体。
    fn encode_response(&self, resp: &LlmResponse) -> Result<Bytes, AdaptorError> {
        let body = json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": encode_parts(&resp.outputs),
                },
                "finishReason": gemini_finish_reason_str(&resp.stop_reason),
                "index": 0,
            }],
            "usageMetadata": {
                "promptTokenCount": resp.usage.prompt_tokens,
                "candidatesTokenCount": resp.usage.completion_tokens,
                "totalTokenCount": resp.usage.prompt_tokens + resp.usage.completion_tokens,
                "cachedContentTokenCount": resp.usage.cached_tokens.unwrap_or(0),
            },
        });
        serde_json::to_vec(&body)
            .map(Bytes::from)
            .map_err(|e| AdaptorError::EncodeFailed(e.to_string()))
    }

    /// 一个完整 SSE 帧的 data 负载 → IR 事件（无状态）。
    fn decode_event(&self, data: &str) -> Result<Vec<StreamEvent>, AdaptorError> {
        let trimmed = data.trim();
        if trimmed.is_empty() || trimmed == "[DONE]" {
            return Ok(Vec::new());
        }
        let v: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut out = Vec::new();

        if let Some(candidates) = v.get("candidates").and_then(Value::as_array) {
            for (ci, candidate) in candidates.iter().enumerate() {
                let ci = ci as u32;
                if let Some(parts) = candidate
                    .get("content")
                    .and_then(|c| c.get("parts"))
                    .and_then(Value::as_array)
                {
                    for (pi, part) in parts.iter().enumerate() {
                        // 每个 part 占一个 index：Gemini 本身没有块索引概念，但下游
                        // （Claude 的 content_block、Responses 的 output_index）都按
                        // index 归属，全部塞进 0 会让多 part 互相覆盖。
                        let index = ci * 1000 + pi as u32;
                        match part {
                            _ if part.get("text").is_some() => {
                                if let Some(text) = part.get("text").and_then(Value::as_str)
                                    && !text.is_empty()
                                {
                                    out.push(StreamEvent::TextDelta {
                                        index,
                                        text: text.to_string(),
                                    });
                                }
                            }
                            _ if part.get("functionCall").is_some() => {
                                let fc = part.get("functionCall").cloned().unwrap_or(json!({}));
                                let name =
                                    fc.get("name").and_then(Value::as_str).unwrap_or_default();
                                out.push(StreamEvent::ToolCallStart {
                                    index,
                                    // Gemini 不给工具调用 id：合成一个确定性的（不能用
                                    // uuid，否则重放同一条流会得到不同 id）。
                                    id: format!("gemini_call_{index}"),
                                    name: name.to_string(),
                                });
                                // Gemini 的 args 是完整对象（非增量），一次性给出整个 JSON。
                                if let Some(args) = fc.get("args") {
                                    out.push(StreamEvent::ToolCallDelta {
                                        index,
                                        arguments_delta: serde_json::to_string(args)
                                            .unwrap_or_else(|_| "{}".into()),
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }

                if let Some(reason) = candidate.get("finishReason").and_then(Value::as_str) {
                    out.push(StreamEvent::MessageStop {
                        reason: gemini_finish_reason(reason),
                    });
                }
            }
        }

        if let Some(usage) = v.get("usageMetadata") {
            let u = decode_usage(Some(usage));
            if u.prompt_tokens > 0 || u.completion_tokens > 0 {
                out.push(StreamEvent::MessageDelta { usage: Some(u) });
            }
        }

        Ok(out)
    }

    fn stream_encoder(&self) -> Box<dyn StreamEncoder> {
        Box::new(GeminiStreamEncoder::new())
    }
}

// ---------- parts 编解码 ----------

/// 单个 Gemini part → IR block。
fn decode_part(part: &Value) -> ContentBlock {
    if let Some(text) = part.get("text").and_then(Value::as_str) {
        return ContentBlock::Text {
            text: text.to_string(),
        };
    }
    if let Some(fc) = part.get("functionCall") {
        return ContentBlock::ToolUse {
            // Gemini 不给 id，合成确定性的。
            id: format!(
                "gemini_call_{}",
                fc.get("name").and_then(Value::as_str).unwrap_or("unknown")
            ),
            name: fc
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            input: fc.get("args").cloned().unwrap_or(json!({})),
        };
    }
    if let Some(inline) = part.get("inline_data").or_else(|| part.get("inlineData")) {
        return ContentBlock::Image {
            media_type: inline
                .get("mime_type")
                .or_else(|| inline.get("mimeType"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            data: inline
                .get("data")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        };
    }
    if let Some(fr) = part.get("functionResponse") {
        return ContentBlock::ToolResult {
            tool_use_id: format!(
                "gemini_call_{}",
                fr.get("name").and_then(Value::as_str).unwrap_or("unknown")
            ),
            content: fr
                .get("response")
                .map(|r| r.to_string())
                .unwrap_or_default(),
        };
    }
    ContentBlock::Unknown { raw: part.clone() }
}

/// IR blocks → Gemini parts。
fn encode_parts(blocks: &[ContentBlock]) -> Vec<Value> {
    blocks
        .iter()
        .map(|b| match b {
            ContentBlock::Text { text } => json!({"text": text}),
            ContentBlock::ToolUse { name, input, .. } => {
                json!({"functionCall": {"name": name, "args": input}})
            }
            ContentBlock::ToolResult { content, .. } => json!({
                "functionResponse": {
                    "name": "unknown",
                    "response": {"content": content},
                },
            }),
            ContentBlock::Image { media_type, data } => json!({
                "inline_data": {"mime_type": media_type, "data": data},
            }),
            ContentBlock::Thinking { text } => json!({"text": text}),
            ContentBlock::Refusal { text } => json!({"text": text}),
            ContentBlock::Unknown { raw } => raw.clone(),
        })
        .collect()
}

/// IR 消息 → Gemini content（角色只有 user / model）。
fn encode_content(m: &Message) -> Value {
    let role = match m.role {
        Role::Assistant => "model",
        // tool 结果在 Gemini 里也走 user 侧。
        Role::User | Role::Tool => "user",
    };
    json!({
        "role": role,
        "parts": encode_parts(&m.content),
    })
}

/// Gemini finishReason → IR StopReason。
///
/// 旧实现把大写的 Gemini 枚举原样透传进 OpenAI 形状，客户端认不出；这张表是
/// 权威映射（对照 `todo/ref/ferryllm/src/adapters/gemini.rs` 的实现）。
fn gemini_finish_reason(reason: &str) -> StopReason {
    match reason {
        // STOP 与 OTHER 之类都视为正常结束。
        "STOP" => StopReason::Stop,
        "MAX_TOKENS" => StopReason::Length,
        "MALFORMED_FUNCTION_CALL" => StopReason::ToolUse,
        "SAFETY" | "RECITATION" | "LANGUAGE" | "BLOCKLIST" | "PROHIBITED_CONTENT" | "SPII" => {
            StopReason::ContentFilter
        }
        other => StopReason::Other(other.to_string()),
    }
}

/// IR StopReason → Gemini finishReason（与上表互逆）。
fn gemini_finish_reason_str(reason: &StopReason) -> String {
    match reason {
        StopReason::Stop => "STOP".into(),
        StopReason::Length => "MAX_TOKENS".into(),
        StopReason::ToolUse => "MALFORMED_FUNCTION_CALL".into(),
        StopReason::ContentFilter => "SAFETY".into(),
        StopReason::Error => "OTHER".into(),
        StopReason::Other(s) => s.clone(),
    }
}

fn decode_usage(usage: Option<&Value>) -> Usage {
    let u = usage.unwrap_or(&Value::Null);
    Usage {
        prompt_tokens: u
            .get("promptTokenCount")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        completion_tokens: u
            .get("candidatesTokenCount")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        cached_tokens: u.get("cachedContentTokenCount").and_then(Value::as_u64),
    }
}

// ---------- 流式编码器 ----------

/// Gemini 流编码器：全程复用同一个 chunk id，并累积 functionCall 参数。
///
/// Gemini 的 `functionCall.args` 是**完整对象**（不是增量碎片），所以这里要把 IR
/// 的 `ToolCallDelta` 碎片累积起来，到收尾时吐一个完整的 functionCall part。
/// 累积状态正是编码侧必须有状态的第二个理由（第一个是 id 复用）。
struct GeminiStreamEncoder {
    id: String,
    model: String,
    /// 各 index 累积的 tool 调用：name 与已拼好的参数串。
    tools: std::collections::BTreeMap<u32, (String, String)>,
    /// 是否已吐过带 usage 的收尾帧。
    finished: bool,
}

impl GeminiStreamEncoder {
    fn new() -> Self {
        Self {
            // 一次性的流 id：全程复用（旧实现每块现生成 uuid，是 G1 记录的反例）。
            id: format!("chatcmpl-{}", uuid::Uuid::new_v4().simple()),
            model: String::new(),
            tools: std::collections::BTreeMap::new(),
            finished: false,
        }
    }

    fn frame(payload: &Value) -> Bytes {
        Bytes::from(format!("data: {payload}\n\n"))
    }

    /// Gemini 流式帧的形状：candidates[0].content.parts。
    fn candidates_frame(&self, parts: Vec<Value>, finish_reason: Value) -> Value {
        json!({
            "id": self.id,
            "model": self.model,
            "candidates": [{
                "content": {"role": "model", "parts": parts},
                "index": 0,
                "finishReason": finish_reason,
            }],
        })
    }
}

impl StreamEncoder for GeminiStreamEncoder {
    fn encode_event(&mut self, ev: &StreamEvent) -> Result<Vec<Bytes>, AdaptorError> {
        match ev {
            StreamEvent::MessageStart { id, model } => {
                if !id.is_empty() {
                    self.id = id.clone();
                }
                self.model = model.clone();
                Ok(Vec::new())
            }
            StreamEvent::TextDelta { text, .. } => Ok(vec![Self::frame(
                &self.candidates_frame(vec![json!({"text": text})], Value::Null),
            )]),
            StreamEvent::ThinkingDelta { text, .. } => {
                // Gemini 的思考内容也用 text part 承载（thought 标记可选）。
                Ok(vec![Self::frame(&self.candidates_frame(
                    vec![json!({"text": text, "thought": true})],
                    Value::Null,
                ))])
            }
            StreamEvent::ToolCallStart { index, name, .. } => {
                self.tools
                    .entry(*index)
                    .or_insert_with(|| (name.clone(), String::new()));
                // 参数还没到，先不发帧（Gemini 的 functionCall 必须带完整 args）。
                Ok(Vec::new())
            }
            StreamEvent::ToolCallDelta {
                index,
                arguments_delta,
            } => {
                let entry = self.tools.entry(*index).or_default();
                entry.1.push_str(arguments_delta);
                Ok(Vec::new())
            }
            StreamEvent::MessageDelta { usage: Some(u) } => {
                // usage 帧的 choices 不能为空（旧实现的 `choices: []` 是记录在案的反例）；
                // Gemini 形状用 candidates，但这里也补一个空 candidates 项之外的 usage 位。
                let mut payload = self.candidates_frame(Vec::new(), Value::Null);
                payload["usageMetadata"] = json!({
                    "promptTokenCount": u.prompt_tokens,
                    "candidatesTokenCount": u.completion_tokens,
                    "totalTokenCount": u.prompt_tokens + u.completion_tokens,
                });
                Ok(vec![Self::frame(&payload)])
            }
            StreamEvent::MessageDelta { usage: None } => Ok(Vec::new()),
            StreamEvent::MessageStop { reason } => {
                let mut out = Vec::new();
                // 把累积的 tool 调用一次性吐出去。
                let parts: Vec<Value> = self
                    .tools
                    .iter()
                    .map(|(_, (name, args))| {
                        let parsed: Value =
                            serde_json::from_str(args).unwrap_or_else(|_| json!({}));
                        json!({"functionCall": {"name": name, "args": parsed}})
                    })
                    .collect();
                if !parts.is_empty() {
                    out.push(Self::frame(&self.candidates_frame(parts, Value::Null)));
                }
                out.push(Self::frame(&self.candidates_frame(
                    Vec::new(),
                    json!(gemini_finish_reason_str(reason)),
                )));
                self.finished = true;
                Ok(out)
            }
            StreamEvent::Error { message } => Ok(vec![Self::frame(&json!({
                "error": {"code": 500, "message": message, "status": "INTERNAL"},
            }))]),
        }
    }

    fn finish(&mut self) -> Result<Vec<Bytes>, AdaptorError> {
        if self.finished {
            return Ok(Vec::new());
        }
        self.finished = true;
        // 未收到 MessageStop 就断流：补一个 STOP 收尾，否则客户端一直挂着。
        Ok(vec![Self::frame(
            &self.candidates_frame(Vec::new(), json!("STOP")),
        )])
    }
}
