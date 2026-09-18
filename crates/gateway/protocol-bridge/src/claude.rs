//! Claude (Anthropic Messages) 格式 codec —— IR ↔ `/v1/messages`（WP3）。
//!
//! 单格式双向 codec：`decode_*` 把 Claude 形状解析成 IR，`encode_*` 把 IR 打回
//! Claude 形状。加新格式只需再实现一个 `FormatCodec`，不需要与每个既有格式
//! 各配一个方向的转换器。
//!
//! ## 本模块修掉的三个既有缺陷（G1）
//!
//! 旧实现 `adaptor::convert_claude_event_to_openai` 里 tool 调用是坏的，三处：
//!
//! 1. **`content_block_start` 没有 match 分支**，整个事件被 `_ => {}` 丢掉。tool 的
//!    `id` / `name` 只在这个事件里携带，于是客户端永远收不到工具身份，只拿到一串
//!    无主的 `input_json_delta` 碎片。→ [`ClaudeCodec::decode_event`] 现在产出
//!    [`StreamEvent::ToolCallStart`]。
//! 2. **`input_json_delta` 的 tool index 被硬编码成 `0`**，并行工具调用全部塌到
//!    同一个 index 上互相覆盖。→ 现在用事件自带的 `index`。
//! 3. **`message_start` 里的 input usage 被丢**。→ 现在转成
//!    [`StreamEvent::MessageDelta`]。
//!
//! 需要跨事件状态的只有**编码侧**（要按 block 索引补 `content_block_start` /
//! `content_block_stop` 边界帧），故 [`ClaudeCodec::decode_event`] 保持无状态。

use bytes::Bytes;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

use crate::adaptor::{AdaptorError, Protocol};
use crate::format_codec::{FormatCodec, StreamEncoder};
use crate::ir::{
    ContentBlock, LlmRequest, LlmResponse, Message, Role, SamplingParams, StopReason, StreamEvent,
    TextBlock, ToolChoice, ToolDef, Usage,
};

/// Claude `max_tokens` 是必填项；IR 未携带时用这个默认值（与旧实现一致）。
const DEFAULT_MAX_TOKENS: u64 = 4096;

/// Claude Messages 格式 codec。
#[derive(Debug, Default, Clone, Copy)]
pub struct ClaudeCodec;

impl ClaudeCodec {
    /// 新建 codec（无状态，可共享）。
    pub const fn new() -> Self {
        Self
    }
}

impl FormatCodec for ClaudeCodec {
    fn format(&self) -> Protocol {
        Protocol::Claude
    }

    /// Claude 请求体 → IR。
    fn decode_request(&self, body: Bytes) -> Result<LlmRequest, AdaptorError> {
        let v: Value =
            serde_json::from_slice(&body).map_err(|e| AdaptorError::DecodeFailed(e.to_string()))?;
        let obj = v.as_object().ok_or_else(|| {
            AdaptorError::DecodeFailed("claude request body is not an object".into())
        })?;

        let system = decode_system(obj.get("system"));
        let mut messages = Vec::new();
        if let Some(arr) = obj.get("messages").and_then(Value::as_array) {
            for m in arr {
                let role = match m.get("role").and_then(Value::as_str) {
                    Some("assistant") => Role::Assistant,
                    // Claude 只有 user/assistant 两个角色；tool_result 走 user 消息。
                    _ => Role::User,
                };
                let content = decode_content(m.get("content"));
                messages.push(Message { role, content });
            }
        }

        let tools = obj
            .get("tools")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| {
                        let name = t.get("name").and_then(Value::as_str)?.to_string();
                        Some(ToolDef {
                            name,
                            description: t
                                .get("description")
                                .and_then(Value::as_str)
                                .map(str::to_string),
                            input_schema: t.get("input_schema").cloned().unwrap_or(json!({})),
                            extra: Map::new(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let sampling = decode_sampling(obj);
        let tool_choice = obj.get("tool_choice").and_then(decode_tool_choice);
        let stream = obj.get("stream").and_then(Value::as_bool).unwrap_or(false);

        // 未建模的顶层键原样保留，避免第一跳就丢掉厂商扩展字段。
        const KNOWN: &[&str] = &[
            "model",
            "messages",
            "system",
            "tools",
            "tool_choice",
            "max_tokens",
            "temperature",
            "top_p",
            "stream",
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

    /// IR → Claude 请求体。
    fn encode_request(&self, req: &LlmRequest) -> Result<Bytes, AdaptorError> {
        let mut out = Map::new();
        out.insert("model".into(), json!(req.model));

        if !req.system.is_empty() {
            out.insert(
                "system".into(),
                Value::Array(
                    req.system
                        .iter()
                        .map(|b| json!({"type": "text", "text": b.text}))
                        .collect(),
                ),
            );
        }

        let messages: Vec<Value> = req
            .messages
            .iter()
            .map(|m| {
                json!({
                    "role": claude_role(&m.role),
                    "content": encode_content(&m.content),
                })
            })
            .collect();
        out.insert("messages".into(), Value::Array(messages));

        if !req.tools.is_empty() {
            let tools: Vec<Value> = req
                .tools
                .iter()
                .map(|t| {
                    let mut tool = json!({
                        "name": t.name,
                        "input_schema": t.input_schema,
                    });
                    if let Some(d) = &t.description {
                        tool["description"] = json!(d);
                    }
                    tool
                })
                .collect();
            out.insert("tools".into(), Value::Array(tools));
        }

        if let Some(tc) = &req.tool_choice
            && let Some(v) = encode_tool_choice(tc)
        {
            out.insert("tool_choice".into(), v);
        }

        // max_tokens 是 Claude 的必填项：IR 缺失时必须补默认值，否则上游直接 400。
        let empty = SamplingParams {
            temperature: None,
            top_p: None,
            max_tokens: None,
            extra: Map::new(),
        };
        let sampling = req.sampling.as_ref().unwrap_or(&empty);
        out.insert(
            "max_tokens".into(),
            json!(sampling.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS)),
        );
        if let Some(t) = sampling.temperature {
            out.insert("temperature".into(), json!(t));
        }
        if let Some(p) = sampling.top_p {
            out.insert("top_p".into(), json!(p));
        }
        if let Some(stop) = sampling.extra.get("stop") {
            // Claude 叫 stop_sequences；字符串要包成数组。
            let seqs = match stop {
                Value::String(s) => json!([s]),
                other => other.clone(),
            };
            out.insert("stop_sequences".into(), seqs);
        }

        out.insert("stream".into(), json!(req.stream));

        // 请求侧未建模字段原样带回。
        for (k, v) in &req.extra {
            out.entry(k.clone()).or_insert_with(|| v.clone());
        }

        serde_json::to_vec(&Value::Object(out))
            .map(Bytes::from)
            .map_err(|e| AdaptorError::EncodeFailed(e.to_string()))
    }

    /// Claude 非流式响应体 → IR。
    fn decode_response(&self, body: Bytes) -> Result<LlmResponse, AdaptorError> {
        let v: Value =
            serde_json::from_slice(&body).map_err(|e| AdaptorError::DecodeFailed(e.to_string()))?;

        let outputs = decode_content(v.get("content"));
        let usage = decode_usage(v.get("usage"));
        let stop_reason = v
            .get("stop_reason")
            .and_then(Value::as_str)
            .map(claude_stop_reason)
            .unwrap_or(StopReason::Stop);

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
            outputs,
            usage,
            stop_reason,
        })
    }

    /// IR → Claude 非流式响应体。
    fn encode_response(&self, resp: &LlmResponse) -> Result<Bytes, AdaptorError> {
        let body = json!({
            "id": resp.id,
            "type": "message",
            "role": "assistant",
            "model": resp.model,
            "content": encode_content(&resp.outputs),
            "stop_reason": claude_stop_reason_str(&resp.stop_reason),
            "usage": {
                "input_tokens": resp.usage.prompt_tokens,
                "output_tokens": resp.usage.completion_tokens,
            },
        });
        serde_json::to_vec(&body)
            .map(Bytes::from)
            .map_err(|e| AdaptorError::EncodeFailed(e.to_string()))
    }

    /// 一个完整 SSE 帧的 data 负载 → IR 事件（无状态，逐事件 1:1）。
    fn decode_event(&self, data: &str) -> Result<Vec<StreamEvent>, AdaptorError> {
        let v: Value = match serde_json::from_str(data) {
            Ok(v) => v,
            // SSE 流里存在非 JSON 控制帧；静默跳过而不是让整条流失败。
            Err(_) => return Ok(Vec::new()),
        };
        let Some(kind) = v.get("type").and_then(Value::as_str) else {
            return Ok(Vec::new());
        };

        let events = match kind {
            "message_start" => {
                let msg = v.get("message").cloned().unwrap_or(json!({}));
                let mut out = vec![StreamEvent::MessageStart {
                    id: msg
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    model: msg
                        .get("model")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                }];
                // message_start 携带 input usage —— 旧实现整块丢弃，计量因此少了 prompt 侧。
                let usage = decode_usage(msg.get("usage"));
                if usage.prompt_tokens > 0 || usage.completion_tokens > 0 {
                    out.push(StreamEvent::MessageDelta { usage: Some(usage) });
                }
                out
            }
            "content_block_start" => {
                let index = event_index(&v);
                let block = v.get("content_block").cloned().unwrap_or(json!({}));
                // tool_use 的 id/name 只在这里出现；旧实现没有这个分支，直接丢事件。
                match block.get("type").and_then(Value::as_str) {
                    Some("tool_use") => vec![StreamEvent::ToolCallStart {
                        index,
                        id: block
                            .get("id")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        name: block
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                    }],
                    // text / thinking block 的起始帧不携带内容，内容全在 delta 里。
                    _ => Vec::new(),
                }
            }
            "content_block_delta" => {
                let index = event_index(&v);
                let delta = v.get("delta").cloned().unwrap_or(json!({}));
                match delta.get("type").and_then(Value::as_str) {
                    Some("text_delta") => vec![StreamEvent::TextDelta {
                        index,
                        text: delta
                            .get("text")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                    }],
                    // index 必须取事件自带值：旧实现硬编码 0，并行工具调用会互相覆盖。
                    Some("input_json_delta") => vec![StreamEvent::ToolCallDelta {
                        index,
                        arguments_delta: delta
                            .get("partial_json")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                    }],
                    Some("thinking_delta") => vec![StreamEvent::ThinkingDelta {
                        index,
                        text: delta
                            .get("thinking")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                    }],
                    // signature_delta 是 thinking 的校验数据，IR 不建模。
                    _ => Vec::new(),
                }
            }
            "message_delta" => {
                let mut out = Vec::new();
                let delta = v.get("delta").cloned().unwrap_or(json!({}));
                if let Some(reason) = delta.get("stop_reason").and_then(Value::as_str) {
                    out.push(StreamEvent::MessageStop {
                        reason: claude_stop_reason(reason),
                    });
                }
                let usage = decode_usage(v.get("usage"));
                if usage.prompt_tokens > 0 || usage.completion_tokens > 0 {
                    out.push(StreamEvent::MessageDelta { usage: Some(usage) });
                }
                out
            }
            "error" => {
                let message = v
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("claude stream error")
                    .to_string();
                vec![StreamEvent::Error { message }]
            }
            // content_block_stop / message_stop / ping：无对应 IR 事件，流终止由
            // SSE 的 [DONE] 或 EOF 表达。
            _ => Vec::new(),
        };
        Ok(events)
    }

    fn stream_encoder(&self) -> Box<dyn StreamEncoder> {
        Box::new(ClaudeStreamEncoder::new())
    }
}

// ---------- 请求/响应方向的编解码 ----------

/// Claude `system` 字段（字符串或 block 数组）→ IR system blocks。
fn decode_system(system: Option<&Value>) -> Vec<TextBlock> {
    match system {
        Some(Value::String(s)) => vec![TextBlock { text: s.clone() }],
        Some(Value::Array(blocks)) => blocks
            .iter()
            .filter_map(|b| {
                let text = b.get("text").and_then(Value::as_str)?;
                Some(TextBlock {
                    text: text.to_string(),
                })
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Claude `content`（字符串或 block 数组）→ IR content blocks。
fn decode_content(content: Option<&Value>) -> Vec<ContentBlock> {
    match content {
        Some(Value::String(s)) => vec![ContentBlock::Text { text: s.clone() }],
        Some(Value::Array(items)) => items.iter().map(decode_block).collect(),
        _ => Vec::new(),
    }
}

/// 单个 Claude content block → IR block。未知类型整体落 `Unknown` 保留。
fn decode_block(item: &Value) -> ContentBlock {
    match item.get("type").and_then(Value::as_str) {
        Some("text") => ContentBlock::Text {
            text: item
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        },
        Some("tool_use") => ContentBlock::ToolUse {
            id: item
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            name: item
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            input: item.get("input").cloned().unwrap_or(json!({})),
        },
        Some("tool_result") => ContentBlock::ToolResult {
            tool_use_id: item
                .get("tool_use_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            content: extract_result_text(item.get("content")),
        },
        Some("thinking") => ContentBlock::Thinking {
            text: item
                .get("thinking")
                .or_else(|| item.get("text"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        },
        Some("image") => {
            let source = item.get("source").cloned().unwrap_or(json!({}));
            ContentBlock::Image {
                media_type: source
                    .get("media_type")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                data: source
                    .get("data")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            }
        }
        _ => ContentBlock::Unknown { raw: item.clone() },
    }
}

/// IR content blocks → Claude `content` 数组。
fn encode_content(blocks: &[ContentBlock]) -> Value {
    Value::Array(
        blocks
            .iter()
            .map(|b| match b {
                ContentBlock::Text { text } => json!({"type": "text", "text": text}),
                ContentBlock::ToolUse { id, name, input } => {
                    json!({"type": "tool_use", "id": id, "name": name, "input": input})
                }
                ContentBlock::ToolResult {
                    tool_use_id,
                    content,
                } => json!({
                    "type": "tool_result",
                    "tool_use_id": tool_use_id,
                    "content": content,
                }),
                // 请求方向用 "thinking"；响应方向 Claude 也会回 "thinking" 块。
                ContentBlock::Thinking { text } => json!({"type": "thinking", "thinking": text}),
                ContentBlock::Image { media_type, data } => json!({
                    "type": "image",
                    "source": {"type": "base64", "media_type": media_type, "data": data},
                }),
                // Claude 没有 refusal 内容块，降级成文本以免整条消息丢失。
                ContentBlock::Refusal { text } => json!({"type": "text", "text": text}),
                ContentBlock::Unknown { raw } => raw.clone(),
            })
            .collect(),
    )
}

/// Claude 的 `tool_result.content` 可以是字符串或 block 数组，统一取文本。
fn extract_result_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|i| i.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

fn decode_sampling(obj: &Map<String, Value>) -> Option<SamplingParams> {
    let temperature = obj.get("temperature").and_then(Value::as_f64);
    let top_p = obj.get("top_p").and_then(Value::as_f64);
    let max_tokens = obj.get("max_tokens").and_then(Value::as_u64);

    let mut extra = Map::new();
    if let Some(stop) = obj.get("stop_sequences") {
        extra.insert("stop".into(), stop.clone());
    }
    if let Some(top_k) = obj.get("top_k") {
        extra.insert("top_k".into(), top_k.clone());
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
    match v.get("type").and_then(Value::as_str) {
        Some("auto") => Some(ToolChoice::Auto),
        Some("any") => Some(ToolChoice::Required),
        Some("tool") => v
            .get("name")
            .and_then(Value::as_str)
            .map(|n| ToolChoice::Specific(n.to_string())),
        Some("none") => Some(ToolChoice::None),
        // 未知形状原样保留，不静默降级成 auto。
        _ => Some(ToolChoice::Raw(v.clone())),
    }
}

fn encode_tool_choice(tc: &ToolChoice) -> Option<Value> {
    match tc {
        ToolChoice::Auto => Some(json!({"type": "auto"})),
        ToolChoice::Required => Some(json!({"type": "any"})),
        ToolChoice::Specific(name) => Some(json!({"type": "tool", "name": name})),
        // None 用省略表达（Claude 的 "none" 较新，省略更兼容）。
        ToolChoice::None => None,
        ToolChoice::Raw(v) => Some(v.clone()),
    }
}

fn claude_role(role: &Role) -> &'static str {
    match role {
        Role::Assistant => "assistant",
        // Claude 用 user 消息承载 tool_result。
        Role::User | Role::Tool => "user",
    }
}

fn decode_usage(usage: Option<&Value>) -> Usage {
    let u = usage.unwrap_or(&Value::Null);
    Usage {
        prompt_tokens: u.get("input_tokens").and_then(Value::as_u64).unwrap_or(0),
        completion_tokens: u.get("output_tokens").and_then(Value::as_u64).unwrap_or(0),
        cached_tokens: u.get("cache_read_input_tokens").and_then(Value::as_u64),
    }
}

fn claude_stop_reason(reason: &str) -> StopReason {
    match reason {
        "end_turn" | "stop_sequence" => StopReason::Stop,
        "max_tokens" => StopReason::Length,
        "tool_use" => StopReason::ToolUse,
        "refusal" => StopReason::ContentFilter,
        other => StopReason::Other(other.to_string()),
    }
}

fn claude_stop_reason_str(reason: &StopReason) -> String {
    match reason {
        StopReason::Stop => "end_turn".into(),
        StopReason::Length => "max_tokens".into(),
        StopReason::ToolUse => "tool_use".into(),
        StopReason::ContentFilter => "refusal".into(),
        StopReason::Error => "end_turn".into(),
        StopReason::Other(s) => s.clone(),
    }
}

fn event_index(v: &Value) -> u32 {
    v.get("index")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .try_into()
        .unwrap_or(u32::MAX)
}

// ---------- 流式编码器 ----------

/// 一个已开启 block 的编码状态。
#[derive(Debug, Clone, Copy, PartialEq)]
enum BlockKind {
    Text,
    Thinking,
    Tool,
}

#[derive(Debug, Clone)]
struct BlockState {
    kind: BlockKind,
    /// tool block 是否已经吐过 `content_block_start`。
    opened: bool,
    id: String,
    name: String,
}

/// Claude 流编码器：持有 block 索引状态，补 Claude 协议要求的边界帧。
///
/// Claude 的事件模型要求每个 block 先 `content_block_start`、delta 若干、
/// 最后 `content_block_stop`；IR 的事件流没有这些边界（它只有带 index 的 delta），
/// 所以边界由本编码器按需补齐——这也是「编码侧有状态」的原因。
struct ClaudeStreamEncoder {
    id: Option<String>,
    started: bool,
    blocks: BTreeMap<u32, BlockState>,
    stopped: bool,
}

impl ClaudeStreamEncoder {
    fn new() -> Self {
        Self {
            id: None,
            started: false,
            blocks: BTreeMap::new(),
            stopped: false,
        }
    }

    /// Claude SSE 帧：`event: <type>` + `data: <json>` + 空行。
    fn frame(event: &str, payload: &Value) -> Bytes {
        Bytes::from(format!("event: {event}\ndata: {payload}\n\n"))
    }

    /// 必要时补 `message_start`（IR 流可能没有显式的 MessageStart）。
    fn ensure_started(&mut self) -> Vec<Bytes> {
        if self.started {
            return Vec::new();
        }
        self.started = true;
        let id = self
            .id
            .clone()
            .unwrap_or_else(|| format!("msg_{}", uuid::Uuid::new_v4().simple()));
        self.id = Some(id.clone());
        vec![Self::frame(
            "message_start",
            &json!({
                "type": "message_start",
                "message": {
                    "id": id,
                    "type": "message",
                    "role": "assistant",
                    "model": "",
                    "content": [],
                    "stop_reason": null,
                    "usage": {"input_tokens": 0, "output_tokens": 0},
                }
            }),
        )]
    }

    /// 确保某 index 的 block 已开启，返回 `content_block_start` 帧（首次才吐）。
    fn ensure_block(&mut self, index: u32, kind: BlockKind, id: &str, name: &str) -> Vec<Bytes> {
        let entry = self.blocks.entry(index).or_insert_with(|| BlockState {
            kind,
            opened: false,
            id: id.to_string(),
            name: name.to_string(),
        });
        // 后续事件可能补上更完整的信息。
        if !id.is_empty() {
            entry.id = id.to_string();
        }
        if !name.is_empty() {
            entry.name = name.to_string();
        }
        if entry.opened {
            return Vec::new();
        }
        entry.opened = true;
        let content_block = match entry.kind {
            BlockKind::Text => json!({"type": "text", "text": ""}),
            BlockKind::Thinking => json!({"type": "thinking", "thinking": ""}),
            // tool 的 id/name 可能到这一步才齐；缺 id 时给占位值，避免整个调用被丢弃。
            BlockKind::Tool => json!({
                "type": "tool_use",
                "id": if entry.id.is_empty() { format!("toolu_{index}") } else { entry.id.clone() },
                "name": if entry.name.is_empty() { "unknown".to_string() } else { entry.name.clone() },
                "input": {},
            }),
        };
        vec![Self::frame(
            "content_block_start",
            &json!({
                "type": "content_block_start",
                "index": index,
                "content_block": content_block,
            }),
        )]
    }

    /// 关闭所有仍开启的 block，返回对应的 `content_block_stop` 帧。
    fn close_blocks(&mut self) -> Vec<Bytes> {
        let open: Vec<u32> = self
            .blocks
            .iter()
            .filter(|(_, s)| s.opened)
            .map(|(i, _)| *i)
            .collect();
        open.into_iter()
            .map(|index| {
                if let Some(s) = self.blocks.get_mut(&index) {
                    s.opened = false;
                }
                Self::frame(
                    "content_block_stop",
                    &json!({"type": "content_block_stop", "index": index}),
                )
            })
            .collect()
    }

    /// `message_delta` + `message_stop` 收尾帧。
    fn finish_frames(&mut self, reason: StopReason) -> Vec<Bytes> {
        if self.stopped {
            return Vec::new();
        }
        self.stopped = true;
        vec![
            Self::frame(
                "message_delta",
                &json!({
                    "type": "message_delta",
                    "delta": {"stop_reason": claude_stop_reason_str(&reason)},
                }),
            ),
            Self::frame("message_stop", &json!({"type": "message_stop"})),
        ]
    }
}

impl StreamEncoder for ClaudeStreamEncoder {
    fn encode_event(&mut self, ev: &StreamEvent) -> Result<Vec<Bytes>, AdaptorError> {
        let mut out = match ev {
            StreamEvent::MessageStart { id, .. } => {
                if id.is_empty() {
                    self.id = None;
                } else {
                    self.id = Some(id.clone());
                }
                self.ensure_started()
            }
            StreamEvent::TextDelta { index, text } => {
                let mut out = self.ensure_block(*index, BlockKind::Text, "", "");
                out.push(Self::frame(
                    "content_block_delta",
                    &json!({
                        "type": "content_block_delta",
                        "index": index,
                        "delta": {"type": "text_delta", "text": text},
                    }),
                ));
                out
            }
            StreamEvent::ThinkingDelta { index, text } => {
                let mut out = self.ensure_block(*index, BlockKind::Thinking, "", "");
                out.push(Self::frame(
                    "content_block_delta",
                    &json!({
                        "type": "content_block_delta",
                        "index": index,
                        "delta": {"type": "thinking_delta", "thinking": text},
                    }),
                ));
                out
            }
            StreamEvent::ToolCallStart { index, id, name } => {
                self.ensure_block(*index, BlockKind::Tool, id, name)
            }
            StreamEvent::ToolCallDelta {
                index,
                arguments_delta,
            } => {
                // 上游可能只给 delta 而不给 start（缺 id/name）——必须能自己把 block
                // 开出来，否则整个 tool 调用会静默消失。
                let mut out = self.ensure_block(*index, BlockKind::Tool, "", "");
                out.push(Self::frame(
                    "content_block_delta",
                    &json!({
                        "type": "content_block_delta",
                        "index": index,
                        "delta": {"type": "input_json_delta", "partial_json": arguments_delta},
                    }),
                ));
                out
            }
            StreamEvent::MessageDelta { usage } => {
                let mut out = self.ensure_started();
                if let Some(u) = usage {
                    out.push(Self::frame(
                        "message_delta",
                        &json!({
                            "type": "message_delta",
                            "delta": {},
                            "usage": {"output_tokens": u.completion_tokens},
                        }),
                    ));
                }
                out
            }
            StreamEvent::MessageStop { reason } => {
                let mut out = self.ensure_started();
                out.extend(self.close_blocks());
                out.extend(self.finish_frames(reason.clone()));
                out
            }
            StreamEvent::Error { message } => {
                vec![Self::frame(
                    "error",
                    &json!({"type": "error", "error": {"type": "api_error", "message": message}}),
                )]
            }
        };
        // 防御：任何内容事件都应先有 message_start。
        if !self.started && !out.is_empty() {
            let mut head = self.ensure_started();
            head.append(&mut out);
            out = head;
        }
        Ok(out)
    }

    fn finish(&mut self) -> Result<Vec<Bytes>, AdaptorError> {
        if self.stopped {
            return Ok(Vec::new());
        }
        let mut out = self.ensure_started();
        out.extend(self.close_blocks());
        out.extend(self.finish_frames(StopReason::Stop));
        Ok(out)
    }
}
