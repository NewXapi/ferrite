//! OpenAI Responses API 格式 codec —— IR ↔ `/v1/responses`（WP5）。
//!
//! 本 crate 里**首个全新格式**（其余都是从旧 codec 迁移），用来验证两跳抽象的
//! 可扩展性：加一个格式只需实现这一个 trait，不必与既有每个格式各配一对转换器。
//!
//! ## 与 Chat Completions 的关键形状差异
//!
//! 这两个 API 都叫 OpenAI，但形状不同，混用会静默出错：
//!
//! - 工具定义是**扁平**的 `{type:"function", name, description, parameters}`，
//!   不像 Chat 那样包在 `function` 子对象里。
//! - 系统提示叫 `instructions`，不是 messages 里的 system 角色。
//! - 输入叫 `input`（可以是字符串、消息数组、或含 `function_call` /
//!   `function_call_output` 的 items 数组），不是 `messages`。
//! - 输出是 `output[]` items（`message` / `function_call` / `reasoning`），
//!   不是 `choices[]`。
//! - 流式是一套**带生命周期的事件**（`response.created` → `output_item.added` →
//!   `*_delta` → `*_done` → `response.completed`），客户端解析器严格依赖顺序：
//!   先收到 delta 而没收到 `response.created` / `output_item.added` 会直接报错。
//!
//! 因此本模块的 `StreamEncoder` 会按需补齐这些边界事件，这也是编码侧必须有状态的原因。

use bytes::Bytes;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

use crate::adaptor::{AdaptorError, Protocol};
use crate::format_codec::{FormatCodec, StreamEncoder};
use crate::ir::{
    ContentBlock, LlmRequest, LlmResponse, Message, Role, SamplingParams, StopReason, StreamEvent,
    TextBlock, ToolDef, Usage,
};

/// OpenAI Responses API 格式 codec。
#[derive(Debug, Default, Clone, Copy)]
pub struct ResponsesCodec;

impl ResponsesCodec {
    /// 新建 codec（无状态，可共享）。
    pub const fn new() -> Self {
        Self
    }
}

impl FormatCodec for ResponsesCodec {
    fn format(&self) -> Protocol {
        Protocol::OpenAIResp
    }

    /// Responses 请求体 → IR。
    fn decode_request(&self, body: Bytes) -> Result<LlmRequest, AdaptorError> {
        let v: Value =
            serde_json::from_slice(&body).map_err(|e| AdaptorError::DecodeFailed(e.to_string()))?;
        let obj = v.as_object().ok_or_else(|| {
            AdaptorError::DecodeFailed("responses request body is not an object".into())
        })?;

        let mut messages = Vec::new();
        match obj.get("input") {
            // 形态一：裸字符串 = 单条 user 消息。
            Some(Value::String(s)) => messages.push(Message {
                role: Role::User,
                content: vec![ContentBlock::Text { text: s.clone() }],
            }),
            // 形态二/三：数组。元素可能是消息，也可能是 function_call(_output) item。
            Some(Value::Array(items)) => {
                for item in items {
                    decode_input_item(item, &mut messages);
                }
            }
            _ => {}
        }

        // `instructions` 是 Responses 的系统提示位。
        let system = match obj.get("instructions") {
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
        };

        let tools = obj
            .get("tools")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| {
                        // Responses 的 tools 是扁平的：name/parameters 就在顶层。
                        let name = t.get("name").and_then(Value::as_str)?.to_string();
                        Some(ToolDef {
                            name,
                            description: t
                                .get("description")
                                .and_then(Value::as_str)
                                .map(str::to_string),
                            input_schema: t.get("parameters").cloned().unwrap_or(json!({})),
                            extra: Map::new(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut extra = Map::new();
        if let Some(prev) = obj.get("previous_response_id") {
            // 服务端会话状态本仓暂不保存，透传兜底而不是丢掉。
            extra.insert("previous_response_id".into(), prev.clone());
        }
        if let Some(store) = obj.get("store") {
            extra.insert("store".into(), store.clone());
        }
        if let Some(m) = obj.get("max_output_tokens") {
            extra.insert("max_output_tokens".into(), m.clone());
        }

        let temperature = obj.get("temperature").and_then(Value::as_f64);
        let top_p = obj.get("top_p").and_then(Value::as_f64);
        let max_tokens = obj.get("max_output_tokens").and_then(Value::as_u64);
        let sampling = if temperature.is_none() && top_p.is_none() && max_tokens.is_none() {
            None
        } else {
            Some(SamplingParams {
                temperature,
                top_p,
                max_tokens,
                extra: Map::new(),
            })
        };

        Ok(LlmRequest {
            model: obj
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            messages,
            system,
            tools,
            tool_choice: None,
            sampling,
            stream: obj.get("stream").and_then(Value::as_bool).unwrap_or(false),
            extra,
        })
    }

    /// IR → Responses 请求体。
    fn encode_request(&self, req: &LlmRequest) -> Result<Bytes, AdaptorError> {
        let mut out = Map::new();
        out.insert("model".into(), json!(req.model));

        if !req.system.is_empty() {
            let text = req
                .system
                .iter()
                .map(|b| b.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            out.insert("instructions".into(), json!(text));
        }

        let input: Vec<Value> = req.messages.iter().map(encode_input_item).collect();
        out.insert("input".into(), Value::Array(input));

        if !req.tools.is_empty() {
            let tools: Vec<Value> = req
                .tools
                .iter()
                .map(|t| {
                    let mut tool = json!({
                        "type": "function",
                        "name": t.name,
                        "parameters": t.input_schema,
                    });
                    if let Some(d) = &t.description {
                        tool["description"] = json!(d);
                    }
                    tool
                })
                .collect();
            out.insert("tools".into(), Value::Array(tools));
        }

        if let Some(s) = &req.sampling {
            if let Some(t) = s.temperature {
                out.insert("temperature".into(), json!(t));
            }
            if let Some(p) = s.top_p {
                out.insert("top_p".into(), json!(p));
            }
            if let Some(m) = s.max_tokens {
                out.insert("max_output_tokens".into(), json!(m));
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

    /// Responses 非流式响应体 → IR。
    fn decode_response(&self, body: Bytes) -> Result<LlmResponse, AdaptorError> {
        let v: Value =
            serde_json::from_slice(&body).map_err(|e| AdaptorError::DecodeFailed(e.to_string()))?;

        let mut outputs = Vec::new();
        let mut has_tool_call = false;

        if let Some(items) = v.get("output").and_then(Value::as_array) {
            for item in items {
                match item.get("type").and_then(Value::as_str) {
                    Some("message") => {
                        if let Some(content) = item.get("content").and_then(Value::as_array) {
                            for part in content {
                                // Responses 的文本 part 叫 output_text（输入侧是 input_text）。
                                if let Some(t) = part.get("text").and_then(Value::as_str)
                                    && part
                                        .get("type")
                                        .and_then(Value::as_str)
                                        .is_none_or(|k| k == "output_text" || k == "text")
                                {
                                    outputs.push(ContentBlock::Text {
                                        text: t.to_string(),
                                    });
                                }
                            }
                        }
                    }
                    Some("function_call") => {
                        has_tool_call = true;
                        outputs.push(ContentBlock::ToolUse {
                            id: item
                                .get("call_id")
                                .or_else(|| item.get("id"))
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            name: item
                                .get("name")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            // arguments 是 JSON 字符串；解析失败不该让整个响应失败。
                            input: item
                                .get("arguments")
                                .and_then(Value::as_str)
                                .and_then(|s| serde_json::from_str::<Value>(s).ok())
                                .unwrap_or_else(|| json!({})),
                        });
                    }
                    Some("reasoning") => {
                        // 摘要文本进 Thinking；加密内容 IR 不建模，跳过。
                        if let Some(summary) = item.get("summary").and_then(Value::as_array) {
                            for s in summary {
                                if let Some(t) = s.get("text").and_then(Value::as_str)
                                    && !t.is_empty()
                                {
                                    outputs.push(ContentBlock::Thinking {
                                        text: t.to_string(),
                                    });
                                }
                            }
                        }
                    }
                    // 其它 item 类型（如内置工具调用）经逃逸口保留。
                    _ => outputs.push(ContentBlock::Unknown { raw: item.clone() }),
                }
            }
        }

        let truncated = v.get("status").and_then(Value::as_str) == Some("incomplete");
        let stop_reason = if has_tool_call {
            StopReason::ToolUse
        } else if truncated {
            StopReason::Length
        } else {
            StopReason::Stop
        };

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
            usage: decode_usage(v.get("usage")),
            stop_reason,
        })
    }

    /// IR → Responses 非流式响应体。
    fn encode_response(&self, resp: &LlmResponse) -> Result<Bytes, AdaptorError> {
        let mut output = Vec::new();
        let text: String = resp
            .outputs
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        if !text.is_empty() {
            output.push(json!({
                "type": "message",
                "id": format!("msg_{}", resp.id),
                "role": "assistant",
                "status": "completed",
                "content": [{"type": "output_text", "text": text, "annotations": []}],
            }));
        }

        for b in &resp.outputs {
            if let ContentBlock::ToolUse { id, name, input } = b {
                output.push(json!({
                    "type": "function_call",
                    "id": format!("fc_{id}"),
                    "call_id": id,
                    "name": name,
                    "arguments": serde_json::to_string(input).unwrap_or_else(|_| "{}".into()),
                    "status": "completed",
                }));
            }
        }

        let status = match resp.stop_reason {
            StopReason::Length => "incomplete",
            _ => "completed",
        };

        let body = json!({
            "id": resp.id,
            "object": "response",
            "status": status,
            "model": resp.model,
            "output": output,
            "usage": {
                "input_tokens": resp.usage.prompt_tokens,
                "output_tokens": resp.usage.completion_tokens,
                "total_tokens": resp.usage.prompt_tokens + resp.usage.completion_tokens,
                "input_tokens_details": {
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
        if trimmed.is_empty() || trimmed == "[DONE]" {
            return Ok(Vec::new());
        }
        let v: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };
        let Some(kind) = v.get("type").and_then(Value::as_str) else {
            return Ok(Vec::new());
        };

        let out = match kind {
            "response.created" => {
                let resp = v.get("response").cloned().unwrap_or(json!({}));
                vec![StreamEvent::MessageStart {
                    id: resp
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    model: resp
                        .get("model")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                }]
            }
            "response.output_text.delta" => vec![StreamEvent::TextDelta {
                index: event_output_index(&v),
                text: v
                    .get("delta")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            }],
            "response.reasoning_summary_text.delta" | "response.reasoning_text.delta" => {
                vec![StreamEvent::ThinkingDelta {
                    index: event_output_index(&v),
                    text: v
                        .get("delta")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                }]
            }
            "response.output_item.added" => {
                let item = v.get("item").cloned().unwrap_or(json!({}));
                match item.get("type").and_then(Value::as_str) {
                    // function_call item 的开头携带 call_id 与 name——两者只在这里出现，
                    // 丢了客户端就只收到一串无主参数（与 Claude 的 content_block_start 同型）。
                    Some("function_call") => vec![StreamEvent::ToolCallStart {
                        index: event_output_index(&v),
                        id: item
                            .get("call_id")
                            .or_else(|| item.get("id"))
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        name: item
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                    }],
                    _ => Vec::new(),
                }
            }
            "response.function_call_arguments.delta" => vec![StreamEvent::ToolCallDelta {
                index: event_output_index(&v),
                arguments_delta: v
                    .get("delta")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            }],
            "response.completed" | "response.incomplete" => {
                let resp = v.get("response").cloned().unwrap_or(json!({}));
                let mut evs = Vec::new();
                let usage = decode_usage(resp.get("usage"));
                if usage.prompt_tokens > 0 || usage.completion_tokens > 0 {
                    evs.push(StreamEvent::MessageDelta { usage: Some(usage) });
                }
                let reason = if kind == "response.incomplete" {
                    StopReason::Length
                } else {
                    resp.get("output")
                        .and_then(Value::as_array)
                        .map(|items| {
                            items.iter().any(|i| {
                                i.get("type").and_then(Value::as_str) == Some("function_call")
                            })
                        })
                        .filter(|has| *has)
                        .map(|_| StopReason::ToolUse)
                        .unwrap_or(StopReason::Stop)
                };
                evs.push(StreamEvent::MessageStop { reason });
                evs
            }
            "response.failed" => {
                let message = v
                    .get("response")
                    .and_then(|r| r.get("error"))
                    .and_then(|e| e.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("responses stream failed")
                    .to_string();
                vec![StreamEvent::Error { message }]
            }
            // 生命周期里的其余事件（item.done / content_part.* / 各类 done）在 IR 里没有
            // 对应概念——IR 的边界由 index 与 StreamEvent 类型表达。
            _ => Vec::new(),
        };

        Ok(out)
    }

    fn stream_encoder(&self) -> Box<dyn StreamEncoder> {
        Box::new(ResponsesStreamEncoder::new())
    }
}

// ---------- 请求/输入编解码 ----------

/// 一条 input 数组元素 → IR 消息（可能产出多条：items 里的 function_call(_output) 各自成条）。
fn decode_input_item(item: &Value, out: &mut Vec<Message>) {
    match item.get("type").and_then(Value::as_str) {
        Some("function_call") => {
            out.push(Message {
                role: Role::Assistant,
                content: vec![ContentBlock::ToolUse {
                    id: item
                        .get("call_id")
                        .or_else(|| item.get("id"))
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    name: item
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    input: item
                        .get("arguments")
                        .and_then(Value::as_str)
                        .and_then(|s| serde_json::from_str::<Value>(s).ok())
                        .unwrap_or_else(|| json!({})),
                }],
            });
        }
        Some("function_call_output") => {
            out.push(Message {
                role: Role::Tool,
                content: vec![ContentBlock::ToolResult {
                    tool_use_id: item
                        .get("call_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    content: match item.get("output") {
                        Some(Value::String(s)) => s.clone(),
                        Some(other) => other.to_string(),
                        None => String::new(),
                    },
                }],
            });
        }
        // 带 role 的消息形态（第三种 input 形态）。
        _ => {
            let role = match item.get("role").and_then(Value::as_str) {
                Some("assistant") => Role::Assistant,
                Some("tool") => Role::Tool,
                _ => Role::User,
            };
            let content = match item.get("content") {
                Some(Value::String(s)) => vec![ContentBlock::Text { text: s.clone() }],
                Some(Value::Array(parts)) => parts
                    .iter()
                    .filter_map(|p| {
                        // input_text / output_text / text 都算文本。
                        p.get("text")
                            .and_then(Value::as_str)
                            .map(|t| ContentBlock::Text {
                                text: t.to_string(),
                            })
                    })
                    .collect(),
                _ => Vec::new(),
            };
            out.push(Message { role, content });
        }
    }
}

/// IR 消息 → Responses input 元素。
fn encode_input_item(m: &Message) -> Value {
    // tool 结果在 Responses 里是独立 item，不是消息内容。
    if m.role == Role::Tool
        && let Some(ContentBlock::ToolResult {
            tool_use_id,
            content,
        }) = m.content.first()
    {
        return json!({
            "type": "function_call_output",
            "call_id": tool_use_id,
            "output": content,
        });
    }

    let role = match m.role {
        Role::Assistant => "assistant",
        Role::Tool | Role::User => "user",
    };

    let text: String = m
        .content
        .iter()
        .filter_map(|b| match b {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");

    json!({
        "type": "message",
        "role": role,
        "content": [{"type": "input_text", "text": text}],
    })
}

fn decode_usage(usage: Option<&Value>) -> Usage {
    let u = usage.unwrap_or(&Value::Null);
    Usage {
        prompt_tokens: u.get("input_tokens").and_then(Value::as_u64).unwrap_or(0),
        completion_tokens: u.get("output_tokens").and_then(Value::as_u64).unwrap_or(0),
        cached_tokens: u
            .get("input_tokens_details")
            .and_then(|d| d.get("cached_tokens"))
            .and_then(Value::as_u64),
    }
}

fn event_output_index(v: &Value) -> u32 {
    v.get("output_index")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .try_into()
        .unwrap_or(u32::MAX)
}

// ---------- 流式编码器 ----------

/// 一个已开启的 tool item 的编码状态。
///
/// `output_index` 是**本格式的 item 序号**（与文本 item 共用一条自增序列），
/// 不是 IR 的事件 index——两者是不同的命名空间，直接混用会让文本与工具项撞号。
#[derive(Debug, Default, Clone)]
struct ToolState {
    opened: bool,
    item_id: String,
    call_id: String,
    name: String,
    arguments: String,
    output_index: u32,
}

/// Responses 流编码器。
///
/// Responses 是**事件生命周期**协议：客户端解析器假定 `response.created` 先到、
/// 每个 output item 先 `output_item.added` 再收 delta、最后以 `response.completed`
/// （或 `response.incomplete`）收尾。IR 的事件流没有这些边界，所以由本编码器补齐
/// ——这是编码侧需要状态的直接原因（要记住哪些 item 已开、参数累积到什么程度）。
struct ResponsesStreamEncoder {
    id: String,
    model: String,
    created_sent: bool,
    text_index: Option<u32>,
    text: String,
    next_index: u32,
    tools: BTreeMap<u32, ToolState>,
    finished: bool,
}

impl ResponsesStreamEncoder {
    fn new() -> Self {
        Self {
            id: format!("resp_{}", uuid::Uuid::new_v4().simple()),
            model: String::new(),
            created_sent: false,
            text_index: None,
            text: String::new(),
            next_index: 0,
            tools: BTreeMap::new(),
            finished: false,
        }
    }

    /// Responses SSE 帧：`event: <type>` + `data: <json>` + 空行。
    fn frame(kind: &str, payload: &Value) -> Bytes {
        Bytes::from(format!("event: {kind}\ndata: {payload}\n\n"))
    }

    /// 整个 response 的快照（`response.created` / `response.completed` 都带它）。
    fn response_snapshot(&self, status: &str, output: Vec<Value>) -> Value {
        json!({
            "id": self.id,
            "object": "response",
            "created_at": 0,
            "status": status,
            "model": self.model,
            "output": output,
            "parallel_tool_calls": true,
        })
    }

    /// 确保 `response.created` 已发（只发一次）。
    fn ensure_created(&mut self) -> Vec<Bytes> {
        if self.created_sent {
            return Vec::new();
        }
        self.created_sent = true;
        vec![Self::frame(
            "response.created",
            &json!({
                "type": "response.created",
                "response": self.response_snapshot("in_progress", Vec::new()),
            }),
        )]
    }

    /// 确保文本 item 与其 content part 已开，返回相应的 `added` 帧（首次才吐）。
    fn ensure_text_item(&mut self) -> Vec<Bytes> {
        if self.text_index.is_some() {
            return Vec::new();
        }
        let out = self.ensure_created();
        let index = self.next_index;
        self.next_index += 1;
        self.text_index = Some(index);

        let item_id = self.message_item_id(index);
        let mut frames = out;
        frames.push(Self::frame(
            "response.output_item.added",
            &json!({
                "type": "response.output_item.added",
                "output_index": index,
                "item": {
                    "type": "message",
                    "id": item_id,
                    "role": "assistant",
                    "status": "in_progress",
                    "content": [],
                },
            }),
        ));
        frames.push(Self::frame(
            "response.content_part.added",
            &json!({
                "type": "response.content_part.added",
                "item_id": item_id,
                "output_index": index,
                "content_index": 0,
                "part": {"type": "output_text", "text": "", "annotations": []},
            }),
        ));
        frames
    }

    fn message_item_id(&self, index: u32) -> String {
        format!("msg_{}_{index}", self.id)
    }

    fn tool_item_id(&self, index: u32) -> String {
        format!("fc_{}_{index}", self.id)
    }

    /// 收尾：关闭已开 item，吐 `response.completed` / `response.incomplete`。
    fn finish_stream(&mut self, reason: StopReason) -> Vec<Bytes> {
        if self.finished {
            return Vec::new();
        }
        self.finished = true;

        let mut out = self.ensure_created();
        let mut output = Vec::new();

        // 文本 item：补齐 done 三连（text.done → content_part.done → output_item.done）。
        if let Some(index) = self.text_index {
            let item_id = self.message_item_id(index);
            let item = json!({
                "type": "message",
                "id": item_id,
                "role": "assistant",
                "status": "completed",
                "content": [{"type": "output_text", "text": self.text, "annotations": []}],
            });
            out.push(Self::frame(
                "response.output_text.done",
                &json!({
                    "type": "response.output_text.done",
                    "item_id": item_id,
                    "output_index": index,
                    "content_index": 0,
                    "text": self.text,
                }),
            ));
            out.push(Self::frame(
                "response.content_part.done",
                &json!({
                    "type": "response.content_part.done",
                    "item_id": item_id,
                    "output_index": index,
                    "content_index": 0,
                    "part": {"type": "output_text", "text": self.text, "annotations": []},
                }),
            ));
            out.push(Self::frame(
                "response.output_item.done",
                &json!({
                    "type": "response.output_item.done",
                    "output_index": index,
                    "item": item,
                }),
            ));
            output.push((index, item));
        }

        // tool item：补齐参数 done + output_item.done。
        let tool_indices: Vec<u32> = self.tools.keys().copied().collect();
        for index in tool_indices {
            let state = self.tools.get(&index).cloned().unwrap_or_default();
            if !state.opened {
                continue;
            }
            let item = json!({
                "type": "function_call",
                "id": state.item_id,
                "call_id": state.call_id,
                "name": state.name,
                "arguments": state.arguments,
                "status": "completed",
            });
            out.push(Self::frame(
                "response.function_call_arguments.done",
                &json!({
                    "type": "response.function_call_arguments.done",
                    "item_id": state.item_id,
                    "output_index": index,
                    "name": state.name,
                    "arguments": state.arguments,
                }),
            ));
            out.push(Self::frame(
                "response.output_item.done",
                &json!({
                    "type": "response.output_item.done",
                    "output_index": index,
                    "item": item,
                }),
            ));
            output.push((index, item));
        }

        output.sort_by_key(|(i, _)| *i);
        let items: Vec<Value> = output.into_iter().map(|(_, v)| v).collect();

        // 截断与工具调用是两种不同的终止原因，客户端据此决定是否续跑。
        let (event_type, status) = if reason == StopReason::Length {
            ("response.incomplete", "incomplete")
        } else {
            ("response.completed", "completed")
        };
        out.push(Self::frame(event_type, &{
            let mut payload = json!({
                "type": event_type,
                "response": self.response_snapshot(status, items),
            });
            if status == "incomplete" {
                payload["response"]["incomplete_details"] = json!({"reason": "max_output_tokens"});
            }
            payload
        }));
        out
    }
}

impl StreamEncoder for ResponsesStreamEncoder {
    fn encode_event(&mut self, ev: &StreamEvent) -> Result<Vec<Bytes>, AdaptorError> {
        match ev {
            StreamEvent::MessageStart { id, model } => {
                if !id.is_empty() {
                    self.id = id.clone();
                }
                self.model = model.clone();
                Ok(self.ensure_created())
            }
            StreamEvent::TextDelta { text, .. } => {
                let mut out = self.ensure_text_item();
                self.text.push_str(text);
                // ensure_text_item 刚刚保证了 Some——若为 None 是编码器内部不变量被破坏，
                // 用 expect 显式暴露而不是静默落到 index 0 撞工具项的序号。
                let index = self
                    .text_index
                    .expect("ensure_text_item must set text_index");
                out.push(Self::frame(
                    "response.output_text.delta",
                    &json!({
                        "type": "response.output_text.delta",
                        "item_id": self.message_item_id(index),
                        "output_index": index,
                        "content_index": 0,
                        "delta": text,
                    }),
                ));
                Ok(out)
            }
            StreamEvent::ThinkingDelta { .. } => {
                // Responses 的 reasoning item 形状较重（summary part + 加密内容），
                // IR 侧的 ThinkingDelta 不足以还原它；此处不开 item，只保证 created 已发，
                // 避免产出半截 item 让客户端解析失败。
                Ok(self.ensure_created())
            }
            StreamEvent::ToolCallStart { index, id, name } => {
                let mut out = self.ensure_created();
                let output_index = match self.tools.get(index) {
                    Some(s) => s.output_index,
                    None => {
                        let allocated = self.next_index;
                        self.next_index += 1;
                        allocated
                    }
                };
                let item_id = self.tool_item_id(output_index);
                let entry = self.tools.entry(*index).or_insert_with(|| ToolState {
                    opened: false,
                    item_id,
                    call_id: String::new(),
                    name: String::new(),
                    arguments: String::new(),
                    output_index,
                });
                if !id.is_empty() {
                    entry.call_id = id.clone();
                }
                if !name.is_empty() {
                    entry.name = name.clone();
                }
                if !entry.opened {
                    entry.opened = true;
                    out.push(Self::frame(
                        "response.output_item.added",
                        &json!({
                            "type": "response.output_item.added",
                            "output_index": entry.output_index,
                            "item": {
                                "type": "function_call",
                                "id": entry.item_id,
                                "call_id": entry.call_id,
                                "name": entry.name,
                                "arguments": "",
                                "status": "in_progress",
                            },
                        }),
                    ));
                }
                Ok(out)
            }
            StreamEvent::ToolCallDelta {
                index,
                arguments_delta,
            } => {
                // 没有 ToolCallStart 时也要能开 item：上游可能只给参数（与 Claude 侧同型）。
                let mut out = self.ensure_created();
                let output_index = match self.tools.get(index) {
                    Some(s) => s.output_index,
                    None => {
                        let allocated = self.next_index;
                        self.next_index += 1;
                        allocated
                    }
                };
                let item_id = self.tool_item_id(output_index);
                let entry = self.tools.entry(*index).or_insert_with(|| ToolState {
                    opened: false,
                    item_id,
                    call_id: format!("call_{index}"),
                    name: String::new(),
                    arguments: String::new(),
                    output_index,
                });
                if !entry.opened {
                    entry.opened = true;
                    out.push(Self::frame(
                        "response.output_item.added",
                        &json!({
                            "type": "response.output_item.added",
                            "output_index": entry.output_index,
                            "item": {
                                "type": "function_call",
                                "id": entry.item_id,
                                "call_id": entry.call_id,
                                "name": entry.name,
                                "arguments": "",
                                "status": "in_progress",
                            },
                        }),
                    ));
                }
                entry.arguments.push_str(arguments_delta);
                out.push(Self::frame(
                    "response.function_call_arguments.delta",
                    &json!({
                        "type": "response.function_call_arguments.delta",
                        "item_id": entry.item_id,
                        "output_index": entry.output_index,
                        "delta": arguments_delta,
                    }),
                ));
                Ok(out)
            }
            StreamEvent::MessageDelta { usage: Some(_) } => {
                // Responses 把 usage 放在最终的 response 快照里，没有独立的 usage 事件；
                // 这里不发帧，收尾时由 response_snapshot 之外的 usage 字段承载（见下）。
                Ok(Vec::new())
            }
            StreamEvent::MessageDelta { usage: None } => Ok(Vec::new()),
            StreamEvent::MessageStop { reason } => Ok(self.finish_stream(reason.clone())),
            StreamEvent::Error { message } => {
                let mut out = self.ensure_created();
                out.push(Self::frame(
                    "response.failed",
                    &json!({
                        "type": "response.failed",
                        "response": {
                            "id": self.id,
                            "status": "failed",
                            "error": {"code": "server_error", "message": message},
                        },
                    }),
                ));
                // 失败是终态：阻止 finish() 再吐成功事件。
                self.finished = true;
                Ok(out)
            }
        }
    }

    fn finish(&mut self) -> Result<Vec<Bytes>, AdaptorError> {
        if self.finished {
            return Ok(Vec::new());
        }
        Ok(self.finish_stream(StopReason::Stop))
    }
}
