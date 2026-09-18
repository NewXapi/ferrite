//! OpenAI Chat Completions 格式 codec —— IR ↔ `/v1/chat/completions`（WP2）。
//!
//! 单格式双向 codec。Chat Completions 是事实上的中枢形状，因此本模块同时是其它格式
//! 对照的基线：字段命名、usage 位置、tool_calls 装配方式都以这里为准。
//!
//! ## 与旧实现（`adaptor::OpenAiCodec`）的关系
//!
//! 旧 codec 只是「解析一下看要不要注入 `stream_options.include_usage`」的透传壳，
//! 因为入站格式当时被硬编码成 OpenAI。新 codec 是完整的双向转换器：既要把 Chat
//! Completions 解成 IR（供转去别的上游格式），也要能把 IR 打回 Chat Completions
//! （供别的格式的客户端接收）。
//!
//! ## 流式 id 稳定性
//!
//! Chat Completions 的每个 chunk 都带 `id`，客户端（以及计费侧的流式聚合）会把
//! 它们当作同一次补全的标识。旧 Gemini 侧每块现生成一个 uuid，是 G1 记录的反例；
//! 这里由 [`OpenAiStreamEncoder`] 记住首个 id 并全程复用。

use bytes::Bytes;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

use crate::adaptor::{AdaptorError, Protocol};
use crate::format_codec::{FormatCodec, StreamEncoder};
use crate::ir::{
    ContentBlock, LlmRequest, LlmResponse, Message, Role, SamplingParams, StopReason, StreamEvent,
    TextBlock, ToolChoice, ToolDef, Usage,
};

/// OpenAI Chat Completions 格式 codec。
#[derive(Debug, Default, Clone, Copy)]
pub struct OpenAiCodec;

impl OpenAiCodec {
    /// 新建 codec（无状态，可共享）。
    pub const fn new() -> Self {
        Self
    }
}

impl FormatCodec for OpenAiCodec {
    fn format(&self) -> Protocol {
        Protocol::OpenAi
    }

    /// Chat Completions 请求体 → IR。
    fn decode_request(&self, body: Bytes) -> Result<LlmRequest, AdaptorError> {
        let v: Value =
            serde_json::from_slice(&body).map_err(|e| AdaptorError::DecodeFailed(e.to_string()))?;
        let obj = v.as_object().ok_or_else(|| {
            AdaptorError::DecodeFailed("openai request body is not an object".into())
        })?;

        let mut messages = Vec::new();
        let mut system = Vec::new();

        if let Some(arr) = obj.get("messages").and_then(Value::as_array) {
            for m in arr {
                let role = m.get("role").and_then(Value::as_str).unwrap_or("user");
                // system 消息在 IR 里独立成 system blocks（各厂商对它的位置要求不同，
                // 留在 messages 里会让下游每个 codec 都得自己再挑一遍）。
                if role == "system" || role == "developer" {
                    system.extend(decode_system(m.get("content")));
                    continue;
                }
                messages.push(decode_message(m, role));
            }
        }

        let tools = obj
            .get("tools")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| {
                        // 既接受 {type:function, function:{...}} 也接受扁平 {name,...}。
                        let f = t.get("function").unwrap_or(t);
                        let name = f.get("name").and_then(Value::as_str)?.to_string();
                        Some(ToolDef {
                            name,
                            description: f
                                .get("description")
                                .and_then(Value::as_str)
                                .map(str::to_string),
                            input_schema: f.get("parameters").cloned().unwrap_or(json!({})),
                            extra: Map::new(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let sampling = decode_sampling(obj);
        let tool_choice = obj.get("tool_choice").and_then(decode_tool_choice);
        let stream = obj.get("stream").and_then(Value::as_bool).unwrap_or(false);

        // stream_options 是 Chat 特有的控制字段：IR 建模它没有意义（别的格式没有对应
        // 概念），但也不能丢——留在 extra 里，回写时原样带回。
        const KNOWN: &[&str] = &[
            "model",
            "messages",
            "tools",
            "tool_choice",
            "temperature",
            "top_p",
            "max_tokens",
            "stream",
            "stream_options",
        ];
        let mut extra = Map::new();
        for (k, val) in obj {
            if !KNOWN.contains(&k.as_str()) {
                extra.insert(k.clone(), val.clone());
            }
        }

        Ok(LlmRequest {
            model: obj
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            messages,
            system,
            tools,
            tool_choice,
            sampling,
            stream,
            extra,
        })
    }

    /// IR → Chat Completions 请求体。
    fn encode_request(&self, req: &LlmRequest) -> Result<Bytes, AdaptorError> {
        let mut out = Map::new();
        out.insert("model".into(), json!(req.model));

        let mut messages: Vec<Value> = Vec::new();
        // system 回到 messages 首部：Chat Completions 用角色消息承载系统提示。
        for b in &req.system {
            messages.push(json!({"role": "system", "content": b.text}));
        }
        for m in &req.messages {
            messages.push(encode_message(m));
        }
        out.insert("messages".into(), Value::Array(messages));

        if !req.tools.is_empty() {
            let tools: Vec<Value> = req
                .tools
                .iter()
                .map(|t| {
                    let mut f = json!({
                        "name": t.name,
                        "parameters": t.input_schema,
                    });
                    if let Some(d) = &t.description {
                        f["description"] = json!(d);
                    }
                    json!({"type": "function", "function": f})
                })
                .collect();
            out.insert("tools".into(), Value::Array(tools));
        }

        if let Some(tc) = &req.tool_choice
            && let Some(v) = encode_tool_choice(tc)
        {
            out.insert("tool_choice".into(), v);
        }

        if let Some(s) = &req.sampling {
            if let Some(t) = s.temperature {
                out.insert("temperature".into(), json!(t));
            }
            if let Some(p) = s.top_p {
                out.insert("top_p".into(), json!(p));
            }
            if let Some(m) = s.max_tokens {
                out.insert("max_tokens".into(), json!(m));
            }
            for (k, v) in &s.extra {
                out.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }

        out.insert("stream".into(), json!(req.stream));

        // 流式请求注入 stream_options.include_usage：上游默认不回 usage，
        // 计费侧就少了这一整块数据（旧实现的这段语义要保住）。
        let has_stream_options = req.extra.contains_key("stream_options");
        if req.stream && !has_stream_options {
            out.insert("stream_options".into(), json!({"include_usage": true}));
        }

        for (k, v) in &req.extra {
            out.entry(k.clone()).or_insert_with(|| v.clone());
        }

        serde_json::to_vec(&Value::Object(out))
            .map(Bytes::from)
            .map_err(|e| AdaptorError::EncodeFailed(e.to_string()))
    }

    /// Chat Completions 非流式响应体 → IR。
    fn decode_response(&self, body: Bytes) -> Result<LlmResponse, AdaptorError> {
        let v: Value =
            serde_json::from_slice(&body).map_err(|e| AdaptorError::DecodeFailed(e.to_string()))?;

        let choice = v
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .cloned()
            .unwrap_or(json!({}));
        let message = choice.get("message").cloned().unwrap_or(json!({}));

        Ok(LlmResponse {
            id: v
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            model: v
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            outputs: decode_message_content(&message),
            usage: decode_usage(v.get("usage")),
            stop_reason: choice
                .get("finish_reason")
                .and_then(Value::as_str)
                .map(chat_stop_reason)
                .unwrap_or(StopReason::Stop),
        })
    }

    /// IR → Chat Completions 非流式响应体。
    fn encode_response(&self, resp: &LlmResponse) -> Result<Bytes, AdaptorError> {
        let (text, tool_calls) = split_outputs(&resp.outputs);
        let mut message = json!({"role": "assistant"});
        // 有 tool_calls 时 content 允许为 null（OpenAI 的既有形状）。
        if tool_calls.is_empty() {
            message["content"] = json!(text);
        } else {
            message["content"] = json!(text);
            message["tool_calls"] = Value::Array(tool_calls);
        }

        let body = json!({
            "id": resp.id,
            "object": "chat.completion",
            "created": chrono::Utc::now().timestamp(),
            "model": resp.model,
            "choices": [{
                "index": 0,
                "message": message,
                "finish_reason": chat_stop_reason_str(&resp.stop_reason),
            }],
            "usage": {
                "prompt_tokens": resp.usage.prompt_tokens,
                "completion_tokens": resp.usage.completion_tokens,
                "total_tokens": resp.usage.prompt_tokens + resp.usage.completion_tokens,
                "prompt_tokens_details": {
                    "cached_tokens": resp.usage.cached_tokens.unwrap_or(0),
                },
            },
        });
        serde_json::to_vec(&body)
            .map(Bytes::from)
            .map_err(|e| AdaptorError::EncodeFailed(e.to_string()))
    }

    /// 一个完整 SSE 帧的 data 负载 → IR 事件（无状态，逐事件 1:1）。
    fn decode_event(&self, data: &str) -> Result<Vec<StreamEvent>, AdaptorError> {
        let trimmed = data.trim();
        // `[DONE]` 与空行是 SSE 控制帧，不是内容。
        if trimmed.is_empty() || trimmed == "[DONE]" {
            return Ok(Vec::new());
        }
        let v: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut out = Vec::new();

        if let Some(choice) = v
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
        {
            let index = choice
                .get("index")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                .try_into()
                .unwrap_or(0u32);
            let delta = choice.get("delta").cloned().unwrap_or(json!({}));

            if let Some(text) = delta.get("content").and_then(Value::as_str)
                && !text.is_empty()
            {
                out.push(StreamEvent::TextDelta {
                    index,
                    text: text.to_string(),
                });
            }
            if let Some(text) = delta.get("reasoning_content").and_then(Value::as_str)
                && !text.is_empty()
            {
                out.push(StreamEvent::ThinkingDelta {
                    index,
                    text: text.to_string(),
                });
            }

            if let Some(calls) = delta.get("tool_calls").and_then(Value::as_array) {
                for call in calls {
                    // tool_calls 项自带的 index 决定它属于哪个工具调用；硬编码会让
                    // 并行调用串成同一个（G1 记录的反例）。
                    let call_index = call
                        .get("index")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        .try_into()
                        .unwrap_or(0u32);
                    let function = call.get("function").cloned().unwrap_or(json!({}));
                    let id = call.get("id").and_then(Value::as_str).unwrap_or_default();
                    let name = function
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default();

                    // 首块带 id/name（OpenAI 把它们只放在第一个 chunk 里）。
                    if !id.is_empty() || !name.is_empty() {
                        out.push(StreamEvent::ToolCallStart {
                            index: call_index,
                            id: id.to_string(),
                            name: name.to_string(),
                        });
                    }
                    if let Some(args) = function.get("arguments").and_then(Value::as_str)
                        && !args.is_empty()
                    {
                        out.push(StreamEvent::ToolCallDelta {
                            index: call_index,
                            arguments_delta: args.to_string(),
                        });
                    }
                }
            }

            if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
                out.push(StreamEvent::MessageStop {
                    reason: chat_stop_reason(reason),
                });
            }
        }

        // 末帧同时携带 choices 与 usage（stream_options.include_usage 的形状），
        // 两者都要产出——只取 choices 会让计费丢掉整个流式的 usage。
        if let Some(usage) = v.get("usage").filter(|u| !u.is_null()) {
            out.push(StreamEvent::MessageDelta {
                usage: Some(decode_usage(Some(usage))),
            });
        }

        Ok(out)
    }

    fn stream_encoder(&self) -> Box<dyn StreamEncoder> {
        Box::new(OpenAiStreamEncoder::new())
    }
}

// ---------- 消息编解码 ----------

/// Chat 消息 → IR 消息。
fn decode_message(m: &Value, role: &str) -> Message {
    let ir_role = match role {
        "assistant" => Role::Assistant,
        "tool" => Role::Tool,
        _ => Role::User,
    };

    let mut content = decode_message_content(m);

    // tool 角色的消息用 tool_call_id 关联调用（Chat 把它放在消息顶层而非 content 里）。
    if ir_role == Role::Tool
        && let Some(id) = m.get("tool_call_id").and_then(Value::as_str)
    {
        let text = content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        content = vec![ContentBlock::ToolResult {
            tool_use_id: id.to_string(),
            content: text,
        }];
    }

    Message {
        role: ir_role,
        content,
    }
}

/// Chat 消息的 `content`/`tool_calls` → IR blocks。
fn decode_message_content(m: &Value) -> Vec<ContentBlock> {
    let mut blocks = Vec::new();

    match m.get("content") {
        Some(Value::String(s)) if !s.is_empty() => {
            blocks.push(ContentBlock::Text { text: s.clone() });
        }
        Some(Value::Array(parts)) => {
            for p in parts {
                match p.get("type").and_then(Value::as_str) {
                    Some("text") => {
                        if let Some(t) = p.get("text").and_then(Value::as_str) {
                            blocks.push(ContentBlock::Text {
                                text: t.to_string(),
                            });
                        }
                    }
                    Some("image_url") => blocks.push(decode_image(p)),
                    // 未知 block 原样保留，跨跳不丢新形态。
                    _ => blocks.push(ContentBlock::Unknown { raw: p.clone() }),
                }
            }
        }
        _ => {}
    }

    if let Some(calls) = m.get("tool_calls").and_then(Value::as_array) {
        for c in calls {
            let function = c.get("function").cloned().unwrap_or(json!({}));
            // arguments 是 JSON 字符串；解析失败不该让整个请求失败（模型偶尔吐半截）。
            let input = function
                .get("arguments")
                .and_then(Value::as_str)
                .and_then(|s| serde_json::from_str::<Value>(s).ok())
                .unwrap_or_else(|| json!({}));
            blocks.push(ContentBlock::ToolUse {
                id: c
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                name: function
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                input,
            });
        }
    }

    blocks
}

/// `image_url` part → IR 图片块；非 data URL 的远程地址无法用 base64 形状表达，走逃逸口。
fn decode_image(part: &Value) -> ContentBlock {
    let url = part
        .get("image_url")
        .and_then(|i| i.get("url"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    // data URL 形如 `data:image/png;base64,<payload>`。
    if let Some(rest) = url.strip_prefix("data:")
        && let Some((meta, data)) = rest.split_once(',')
    {
        let media_type = meta.split(';').next().unwrap_or("image/png").to_string();
        return ContentBlock::Image {
            media_type,
            data: data.to_string(),
        };
    }
    ContentBlock::Unknown { raw: part.clone() }
}

/// system 字段（字符串或 part 数组）→ IR system blocks。
fn decode_system(content: Option<&Value>) -> Vec<TextBlock> {
    match content {
        Some(Value::String(s)) if !s.is_empty() => vec![TextBlock { text: s.clone() }],
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|p| {
                p.get("text").and_then(Value::as_str).map(|t| TextBlock {
                    text: t.to_string(),
                })
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// IR 消息 → Chat 消息。
fn encode_message(m: &Message) -> Value {
    let role = match m.role {
        Role::Assistant => "assistant",
        Role::Tool => "tool",
        Role::User => "user",
    };

    // tool 结果在 Chat 里是独立角色 + tool_call_id，不是 content block。
    if let Some(ContentBlock::ToolResult {
        tool_use_id,
        content,
    }) = m
        .content
        .iter()
        .find(|b| matches!(b, ContentBlock::ToolResult { .. }))
    {
        return json!({
            "role": "tool",
            "tool_call_id": tool_use_id,
            "content": content,
        });
    }

    let (text, tool_calls) = split_outputs(&m.content);
    let mut out = json!({"role": role});

    if tool_calls.is_empty() {
        out["content"] = json!(text);
    } else {
        // 只有文本时才发字符串 content；否则发 null（OpenAI 的既有形状）。
        out["content"] = if text.is_empty() {
            Value::Null
        } else {
            json!(text)
        };
        out["tool_calls"] = Value::Array(tool_calls);
    }

    // 图片等非文本 block：转成 Chat 的 content 数组形态。
    let images: Vec<Value> = m
        .content
        .iter()
        .filter_map(|b| match b {
            ContentBlock::Image { media_type, data } => Some(json!({
                "type": "image_url",
                "image_url": {"url": format!("data:{media_type};base64,{data}")},
            })),
            _ => None,
        })
        .collect();
    if !images.is_empty() {
        let mut parts = vec![json!({"type": "text", "text": text})];
        parts.extend(images);
        out["content"] = Value::Array(parts);
    }

    out
}

/// 把 IR blocks 拆成 (文本, tool_calls JSON)。
fn split_outputs(blocks: &[ContentBlock]) -> (String, Vec<Value>) {
    let mut text = String::new();
    let mut calls = Vec::new();
    for b in blocks {
        match b {
            ContentBlock::Text { text: t } => text.push_str(t),
            ContentBlock::ToolUse { id, name, input } => calls.push(json!({
                "id": id,
                "type": "function",
                "function": {
                    "name": name,
                    "arguments": serde_json::to_string(input).unwrap_or_else(|_| "{}".into()),
                },
            })),
            _ => {}
        }
    }
    (text, calls)
}

fn decode_sampling(obj: &Map<String, Value>) -> Option<SamplingParams> {
    let temperature = obj.get("temperature").and_then(Value::as_f64);
    let top_p = obj.get("top_p").and_then(Value::as_f64);
    let max_tokens = obj.get("max_tokens").and_then(Value::as_u64);

    let mut extra = Map::new();
    if let Some(stop) = obj.get("stop") {
        extra.insert("stop".into(), stop.clone());
    }

    if temperature.is_none() && top_p.is_none() && max_tokens.is_none() && extra.is_empty() {
        return None;
    }
    Some(SamplingParams {
        temperature,
        top_p,
        max_tokens,
        extra,
    })
}

fn decode_tool_choice(v: &Value) -> Option<ToolChoice> {
    match v {
        Value::String(s) => match s.as_str() {
            "auto" => Some(ToolChoice::Auto),
            "none" => Some(ToolChoice::None),
            "required" => Some(ToolChoice::Required),
            other => Some(ToolChoice::Raw(json!(other))),
        },
        Value::Object(o) => o
            .get("function")
            .and_then(|f| f.get("name"))
            .and_then(Value::as_str)
            .map(|n| ToolChoice::Specific(n.to_string()))
            .or_else(|| Some(ToolChoice::Raw(v.clone()))),
        _ => Some(ToolChoice::Raw(v.clone())),
    }
}

fn encode_tool_choice(tc: &ToolChoice) -> Option<Value> {
    match tc {
        ToolChoice::Auto => Some(json!("auto")),
        ToolChoice::Required => Some(json!("required")),
        ToolChoice::None => Some(json!("none")),
        ToolChoice::Specific(name) => Some(json!({"type": "function", "function": {"name": name}})),
        ToolChoice::Raw(v) => Some(v.clone()),
    }
}

fn decode_usage(usage: Option<&Value>) -> Usage {
    let u = usage.unwrap_or(&Value::Null);
    Usage {
        prompt_tokens: u.get("prompt_tokens").and_then(Value::as_u64).unwrap_or(0),
        completion_tokens: u
            .get("completion_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        cached_tokens: u
            .get("prompt_tokens_details")
            .and_then(|d| d.get("cached_tokens"))
            .and_then(Value::as_u64),
    }
}

fn chat_stop_reason(reason: &str) -> StopReason {
    match reason {
        "stop" => StopReason::Stop,
        "length" => StopReason::Length,
        "tool_calls" | "function_call" => StopReason::ToolUse,
        "content_filter" => StopReason::ContentFilter,
        other => StopReason::Other(other.to_string()),
    }
}

fn chat_stop_reason_str(reason: &StopReason) -> String {
    match reason {
        StopReason::Stop => "stop".into(),
        StopReason::Length => "length".into(),
        StopReason::ToolUse => "tool_calls".into(),
        StopReason::ContentFilter => "content_filter".into(),
        StopReason::Error => "stop".into(),
        StopReason::Other(s) => s.clone(),
    }
}

// ---------- 流式编码器 ----------

/// Chat Completions 流编码器：持有全流稳定的 chunk id，并把 tool 碎片装配成
/// `tool_calls[].function.arguments` 分片。
struct OpenAiStreamEncoder {
    id: String,
    model: String,
    created: i64,
    /// 首次对外输出时是否已发过 role 帧。
    role_sent: bool,
    /// 记下各 tool index 是否已发过 id/name 头帧。
    tool_started: BTreeMap<u32, bool>,
    finished: bool,
}

impl OpenAiStreamEncoder {
    fn new() -> Self {
        Self {
            id: format!("chatcmpl-{}", uuid::Uuid::new_v4().simple()),
            model: String::new(),
            created: chrono::Utc::now().timestamp(),
            role_sent: false,
            tool_started: BTreeMap::new(),
            finished: false,
        }
    }

    /// 一个 SSE 数据帧。
    fn frame(payload: &Value) -> Bytes {
        Bytes::from(format!("data: {payload}\n\n"))
    }

    /// 组装一个 chunk：id/object/created/model 全流一致。
    fn chunk(&self, delta: Value, finish_reason: Value) -> Value {
        json!({
            "id": self.id,
            "object": "chat.completion.chunk",
            "created": self.created,
            "model": self.model,
            "choices": [{
                "index": 0,
                "delta": delta,
                "finish_reason": finish_reason,
            }],
        })
    }
}

impl StreamEncoder for OpenAiStreamEncoder {
    fn encode_event(&mut self, ev: &StreamEvent) -> Result<Vec<Bytes>, AdaptorError> {
        if let StreamEvent::MessageStart { id, model } = ev {
            // 采信上游给的 id/model；没有就保留自生成的 id。
            if !id.is_empty() {
                self.id = id.clone();
            }
            self.model = model.clone();
            self.role_sent = true;
            return Ok(vec![Self::frame(
                &self.chunk(json!({"role": "assistant", "content": ""}), Value::Null),
            )]);
        }

        let mut out = Vec::new();
        // 先补角色帧：Chat 客户端期望首个 chunk 带 role。
        if !self.role_sent {
            self.role_sent = true;
            out.push(Self::frame(
                &self.chunk(json!({"role": "assistant", "content": ""}), Value::Null),
            ));
        }

        match ev {
            StreamEvent::TextDelta { text, .. } => {
                out.push(Self::frame(
                    &self.chunk(json!({"content": text}), Value::Null),
                ));
            }
            StreamEvent::ThinkingDelta { text, .. } => {
                out.push(Self::frame(
                    &self.chunk(json!({"reasoning_content": text}), Value::Null),
                ));
            }
            StreamEvent::ToolCallStart { index, id, name } => {
                self.tool_started.insert(*index, true);
                out.push(Self::frame(&self.chunk(
                    json!({"tool_calls": [{
                        "index": index,
                        "id": id,
                        "type": "function",
                        "function": {"name": name, "arguments": ""},
                    }]}),
                    Value::Null,
                )));
            }
            StreamEvent::ToolCallDelta {
                index,
                arguments_delta,
            } => {
                // 没有 start 也要能装配：补一个不带 id/name 的头帧，否则客户端
                // 拿到的是一串无主参数。
                if !self.tool_started.get(index).copied().unwrap_or(false) {
                    self.tool_started.insert(*index, true);
                    out.push(Self::frame(&self.chunk(
                        json!({"tool_calls": [{
                            "index": index,
                            "type": "function",
                            "function": {"arguments": ""},
                        }]}),
                        Value::Null,
                    )));
                }
                out.push(Self::frame(&self.chunk(
                    json!({"tool_calls": [{
                        "index": index,
                        "function": {"arguments": arguments_delta},
                    }]}),
                    Value::Null,
                )));
            }
            StreamEvent::MessageDelta { usage: Some(u) } => {
                // usage 帧必须带非空 choices：`choices: []` 会被多数客户端拒绝。
                let mut payload = self.chunk(json!({}), Value::Null);
                payload["usage"] = json!({
                    "prompt_tokens": u.prompt_tokens,
                    "completion_tokens": u.completion_tokens,
                    "total_tokens": u.prompt_tokens + u.completion_tokens,
                });
                out.push(Self::frame(&payload));
            }
            StreamEvent::MessageDelta { usage: None } => {}
            StreamEvent::MessageStop { reason } => {
                out.push(Self::frame(
                    &self.chunk(json!({}), json!(chat_stop_reason_str(reason))),
                ));
                out.push(Self::done_frame());
                self.finished = true;
            }
            StreamEvent::Error { message } => {
                out.push(Self::frame(&json!({
                    "error": {"message": message, "type": "api_error"},
                })));
            }
            // MessageStart 已在上面提前返回。
            StreamEvent::MessageStart { .. } => {}
        }

        Ok(out)
    }

    fn finish(&mut self) -> Result<Vec<Bytes>, AdaptorError> {
        // 未发过 [DONE] 就补上：客户端靠它判定流结束，缺了会一直挂着。
        if self.finished {
            return Ok(Vec::new());
        }
        self.finished = true;
        Ok(vec![Self::done_frame()])
    }

    /// 标记上游已发过 [DONE]：调用方（pipeline 层的帧扫描）看到 [DONE] 时调用，
    /// 让 `finish()` 不再补发第二个——OpenAI 客户端会把两个 [DONE] 当两次流终止。
    pub fn mark_done(&mut self) {
        self.finished = true;
    }
}

impl OpenAiStreamEncoder {
    fn done_frame() -> Bytes {
        Bytes::from_static(b"data: [DONE]\n\n")
    }
}
