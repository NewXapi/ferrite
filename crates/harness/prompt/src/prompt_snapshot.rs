//! Rust 端 prompt snapshot 边界。
//!
//! ## 设计
//!
//! 前端（Dioxus / WASM）负责：
//! - 拼好 OpenAI-compatible `messages` + `tools` JSON
//! - 展开所有 `{{...}}` 宏（前端有 macro engine）
//! - 把 snapshot 顶层加上 marker 字段 `_ferrite_agent_prompt_marker`
//!
//! Rust 端负责：
//! - 拒绝任何**未物化**的 snapshot（marker 仍出现在 message content 里 → 拒绝）
//! - 校验 context policy 与 `ResolvedAgentProfile.context` 一致
//! - 把 JSON 解析成 `AgentModelMessage` / `AgentModelContentPart`
//!
//! ## 兼容
//!
//! 上游 TauriTavern 用 `_tauritavern_agent_prompt_marker`；Ferrite 用
//! `_ferrite_agent_prompt_marker`。两条 marker 同样被识别；这样前端切换
//! 上下游时不会因 marker 名差异而误判为「未物化」。
//!
//! ponytail: 解析层用 `serde_json::Value` 而不是结构体 — snapshot 来自前端，
//! 字段命名/嵌套会演化；硬编 `Deserialize` 会在每次 schema 变时锁死整个 crate。

use harness_core::AgentContextPolicy;
use serde_json::Value;
use thiserror::Error;

use crate::types::{AgentModelContentPart, AgentModelMessage, AgentModelRole};

/// Ferrite 端 marker 字段名（顶层 + 每条 message 内的 content）。
pub const AGENT_PROMPT_MARKER_FIELD: &str = "_ferrite_agent_prompt_marker";

/// 上游 TauriTavern 端 marker 字段名；同样识别以保持兼容。
pub const LEGACY_TAURITAVERN_PROMPT_MARKER_FIELD: &str = "_tauritavern_agent_prompt_marker";

/// snapshot 的 kind；用于 `request_from_prompt_snapshot` 决定走哪条解析路径。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentModelSnapshotKind {
    /// 标准 OpenAI chat completion payload（`chatCompletionPayload` / `generateData`）
    ChatCompletion,
    /// 已冻结的 run input snapshot（`frozenRunInputSnapshot`）
    FrozenRunInput,
    /// 未知 / 未指定
    Other,
}

/// snapshot 解析错误。
#[derive(Debug, Error, PartialEq)]
pub enum PromptSnapshotError {
    /// snapshot 包含未物化的 marker；前端必须先展开所有 `{{...}}`
    #[error(
        "prompt_snapshot.unfinalized: marker `{0}` present in snapshot — frontend must materialize all macros first"
    )]
    UnfinalizedMarker(&'static str),
    /// context policy 与 profile 不匹配
    #[error(
        "prompt_snapshot.context_policy_mismatch: snapshot context differs from profile context"
    )]
    ContextPolicyMismatch,
    /// 顶层 JSON 不是 object
    #[error("prompt_snapshot.invalid_top_level: snapshot root must be a JSON object")]
    InvalidTopLevel,
    /// `messages` 不是数组
    #[error("prompt_snapshot.invalid_messages: `messages` must be an array of message objects")]
    InvalidMessages,
    /// 单条 message 缺 role 或 parts
    #[error("prompt_snapshot.invalid_message: missing `role` or `parts` on message index {0}")]
    InvalidMessage(usize),
    /// 单条 part 缺必要字段
    #[error(
        "prompt_snapshot.invalid_part: malformed part on message index {0}, part index {1}: {2}"
    )]
    InvalidPart(usize, usize, &'static str),
    /// marker 顶层存在但值非字符串
    #[error(
        "prompt_snapshot.marker_value_invalid: marker `{0}` must be a string when present at the snapshot top level"
    )]
    MarkerValueInvalid(&'static str),
}

/// 识别 snapshot 的 kind。
pub fn is_prompt_snapshot(value: &Value) -> bool {
    let Some(obj) = value.as_object() else {
        return false;
    };
    obj.contains_key("chatCompletionPayload")
        || obj.contains_key("chat_completion_payload")
        || obj.contains_key("generateData")
        || obj.contains_key("generate_data")
        || obj.contains_key("frozenRunInputSnapshot")
        || obj.contains_key("contextPolicy")
        || obj.contains_key("context_policy")
}

/// 从 snapshot 中识别 kind。
pub fn snapshot_kind(value: &Value) -> AgentModelSnapshotKind {
    let Some(obj) = value.as_object() else {
        return AgentModelSnapshotKind::Other;
    };
    if obj.contains_key("frozenRunInputSnapshot") {
        return AgentModelSnapshotKind::FrozenRunInput;
    }
    if obj.contains_key("chatCompletionPayload")
        || obj.contains_key("chat_completion_payload")
        || obj.contains_key("generateData")
        || obj.contains_key("generate_data")
    {
        return AgentModelSnapshotKind::ChatCompletion;
    }
    AgentModelSnapshotKind::Other
}

/// 拒绝任何带未物化 marker 的 snapshot（含上下游两条 marker）。
///
/// 走两条路径：
/// 1. 顶层 marker：snapshot 必须已被物化（前端 marker 是空字符串或缺失）。
/// 2. message content 内的 marker：拒绝（前端忘记展开 `{{...}}`）。
pub fn reject_unfinalized_snapshot(snapshot: &Value) -> Result<(), PromptSnapshotError> {
    let obj = snapshot
        .as_object()
        .ok_or(PromptSnapshotError::InvalidTopLevel)?;

    // 顶层 marker：值必须是空字符串（即「我已物化」），其他值拒绝
    for key in [
        AGENT_PROMPT_MARKER_FIELD,
        LEGACY_TAURITAVERN_PROMPT_MARKER_FIELD,
    ] {
        if let Some(marker) = obj.get(key) {
            match marker {
                Value::String(s) if s.is_empty() => {}
                Value::String(_) => return Err(PromptSnapshotError::UnfinalizedMarker(key)),
                _ => return Err(PromptSnapshotError::MarkerValueInvalid(key)),
            }
        }
    }

    // message content 内的 marker：任何 `{{...}}` 残留或 marker 字段都拒绝
    if let Some(messages) = obj.get("messages").and_then(Value::as_array) {
        for (mi, msg) in messages.iter().enumerate() {
            check_message_no_marker(msg, mi)?;
        }
    }
    Ok(())
}

fn check_message_no_marker(msg: &Value, msg_index: usize) -> Result<(), PromptSnapshotError> {
    // message 顶层不应有 marker 字段
    if let Some(m) = msg.as_object() {
        for key in [
            AGENT_PROMPT_MARKER_FIELD,
            LEGACY_TAURITAVERN_PROMPT_MARKER_FIELD,
        ] {
            if m.contains_key(key) {
                return Err(PromptSnapshotError::UnfinalizedMarker(key));
            }
        }
    }

    // content 是字符串 → 检查是否含 `{{`
    if let Some(Value::String(s)) = msg.get("content") {
        if s.contains("{{") {
            return Err(PromptSnapshotError::UnfinalizedMarker(
                AGENT_PROMPT_MARKER_FIELD,
            ));
        }
    }

    // content 是数组 → 每个 part 检查
    if let Some(Value::Array(parts)) = msg.get("content") {
        for (pi, part) in parts.iter().enumerate() {
            check_part_no_marker(part, msg_index, pi)?;
        }
    }
    Ok(())
}

fn check_part_no_marker(
    part: &Value,
    _msg_index: usize,
    _part_index: usize,
) -> Result<(), PromptSnapshotError> {
    let Some(obj) = part.as_object() else {
        return Ok(());
    };
    for key in [
        AGENT_PROMPT_MARKER_FIELD,
        LEGACY_TAURITAVERN_PROMPT_MARKER_FIELD,
    ] {
        if obj.contains_key(key) {
            return Err(PromptSnapshotError::UnfinalizedMarker(key));
        }
    }
    if let Some(Value::String(s)) = obj.get("text") {
        if s.contains("{{") {
            return Err(PromptSnapshotError::UnfinalizedMarker(
                AGENT_PROMPT_MARKER_FIELD,
            ));
        }
    }
    Ok(())
}

/// 校验 snapshot 内的 context policy 与 profile 提供的 policy 一致。
///
/// snapshot 顶层可以携带 `contextPolicy` / `context_policy` 字段；如果存在则与
/// `profile` 比对；不存在则跳过（前端没有 override）。
pub fn validate_prompt_snapshot_context_policy(
    snapshot: &Value,
    policy: &AgentContextPolicy,
) -> Result<(), PromptSnapshotError> {
    let obj = match snapshot.as_object() {
        Some(obj) => obj,
        None => return Err(PromptSnapshotError::InvalidTopLevel),
    };

    let snapshot_policy = obj
        .get("contextPolicy")
        .or_else(|| obj.get("context_policy"));

    let Some(snapshot_policy) = snapshot_policy else {
        // 没有 override → 默认通过
        return Ok(());
    };

    // 解析成 AgentContextPolicy 再比对
    let snapshot_policy: AgentContextPolicy = serde_json::from_value(snapshot_policy.clone())
        .map_err(|_| PromptSnapshotError::ContextPolicyMismatch)?;
    if &snapshot_policy != policy {
        return Err(PromptSnapshotError::ContextPolicyMismatch);
    }
    Ok(())
}

/// 从 OpenAI-compatible payload 解析出消息数组。
///
/// 接受以下结构（顶层任一）：
/// - 直接 `messages` 数组
/// - `{ "chatCompletionPayload": { "messages": [...] } }`
/// - `{ "chat_completion_payload": { "messages": [...] } }`
/// - `{ "generateData": { "messages": [...] } }`
pub fn messages_from_payload(
    payload: &Value,
) -> Result<Vec<AgentModelMessage>, PromptSnapshotError> {
    let Some(messages_value) = locate_messages(payload) else {
        // 没有 messages 字段视为空消息列表（不是错误）
        return Ok(Vec::new());
    };
    let arr = messages_value
        .as_array()
        .ok_or(PromptSnapshotError::InvalidMessages)?;
    let mut out = Vec::with_capacity(arr.len());
    for (i, item) in arr.iter().enumerate() {
        out.push(message_from_openai_value(item, i)?);
    }
    Ok(out)
}
fn locate_messages<'a>(value: &'a Value) -> Option<&'a Value> {
    if let Some(messages) = value.get("messages") {
        return Some(messages);
    }
    for key in [
        "chatCompletionPayload",
        "chat_completion_payload",
        "generateData",
        "generate_data",
    ] {
        if let Some(nested) = value.get(key) {
            if let Some(messages) = nested.get("messages") {
                return Some(messages);
            }
        }
    }
    None
}

/// 单条 OpenAI 消息 → `AgentModelMessage`。
pub fn message_from_openai_value(
    value: &Value,
    index: usize,
) -> Result<AgentModelMessage, PromptSnapshotError> {
    let obj = value
        .as_object()
        .ok_or(PromptSnapshotError::InvalidMessage(index))?;

    let role_str = obj
        .get("role")
        .and_then(Value::as_str)
        .ok_or(PromptSnapshotError::InvalidMessage(index))?;
    let role = match role_str {
        "system" => AgentModelRole::System,
        "user" => AgentModelRole::User,
        "assistant" => AgentModelRole::Assistant,
        "tool" => AgentModelRole::Tool,
        _ => return Err(PromptSnapshotError::InvalidPart(index, 0, "unknown role")),
    };

    let name = obj
        .get("name")
        .and_then(Value::as_str)
        .map(|s| s.to_string());

    // OpenAI content 可以是字符串或数组
    let parts = match obj.get("content") {
        Some(Value::String(s)) => vec![AgentModelContentPart::Text { text: s.clone() }],
        Some(Value::Array(arr)) => content_parts_from_openai_value(arr, index)?,
        Some(Value::Null) | None => {
            // assistant tool-call 消息可能 content=null；尝试 tool_calls
            if let Some(Value::Array(calls)) = obj.get("tool_calls") {
                let mut parts = Vec::with_capacity(calls.len());
                for call in calls {
                    parts.push(tool_call_part_from_openai(call, index, parts.len())?);
                }
                parts
            } else {
                return Err(PromptSnapshotError::InvalidMessage(index));
            }
        }
        Some(_) => return Err(PromptSnapshotError::InvalidMessage(index)),
    };

    // user/assistant 消息可能同时含 tool_calls（content 部分已被上面的
    // content 分支处理；tool_calls 单独追加）。
    let mut parts = parts;
    if let Some(Value::Array(calls)) = obj.get("tool_calls") {
        for call in calls {
            parts.push(tool_call_part_from_openai(call, index, parts.len())?);
        }
    }

    if parts.is_empty() {
        return Err(PromptSnapshotError::InvalidMessage(index));
    }

    Ok(AgentModelMessage { role, parts, name })
}

fn tool_call_part_from_openai(
    value: &Value,
    msg_index: usize,
    part_index: usize,
) -> Result<AgentModelContentPart, PromptSnapshotError> {
    let obj = value.as_object().ok_or(PromptSnapshotError::InvalidPart(
        msg_index,
        part_index,
        "tool_call must be object",
    ))?;
    let call_id = obj
        .get("id")
        .and_then(Value::as_str)
        .ok_or(PromptSnapshotError::InvalidPart(
            msg_index,
            part_index,
            "tool_call.id missing",
        ))?
        .to_string();

    // OpenAI: tool_call.function.{name, arguments}
    let function =
        obj.get("function")
            .and_then(Value::as_object)
            .ok_or(PromptSnapshotError::InvalidPart(
                msg_index,
                part_index,
                "tool_call.function missing",
            ))?;
    let model_alias = function
        .get("name")
        .and_then(Value::as_str)
        .ok_or(PromptSnapshotError::InvalidPart(
            msg_index,
            part_index,
            "tool_call.function.name missing",
        ))?
        .to_string();
    let arguments_raw = function.get("arguments");
    let arguments = match arguments_raw {
        Some(Value::String(s)) => serde_json::from_str(s).unwrap_or(Value::String(s.clone())),
        Some(other) => other.clone(),
        None => Value::Null,
    };

    Ok(AgentModelContentPart::ToolCall {
        call_id,
        tool_id: model_alias.clone(), // 解析后由 runtime 解析完整 tool_id；这里先放 alias
        model_alias,
        arguments,
    })
}

/// 把 OpenAI 多模态 `content` 数组解析成 `AgentModelContentPart`。
pub fn content_parts_from_openai_value(
    arr: &[Value],
    msg_index: usize,
) -> Result<Vec<AgentModelContentPart>, PromptSnapshotError> {
    let mut out = Vec::with_capacity(arr.len());
    for (i, item) in arr.iter().enumerate() {
        let Some(obj) = item.as_object() else {
            return Err(PromptSnapshotError::InvalidPart(
                msg_index,
                i,
                "part must be object",
            ));
        };
        match obj.get("type").and_then(Value::as_str) {
            Some("text") => {
                let text = obj
                    .get("text")
                    .and_then(Value::as_str)
                    .ok_or(PromptSnapshotError::InvalidPart(
                        msg_index,
                        i,
                        "text part missing `text`",
                    ))?
                    .to_string();
                out.push(AgentModelContentPart::Text { text });
            }
            Some("image_url") => {
                // MVP：不解析图片细节；把 image_url 序列化成文本标记，runtime 层丢弃
                // 非文本 part（仅 OpenAI-compatible 模型保留图片；这里走安全降级）。
                let url = obj
                    .get("image_url")
                    .and_then(|v| v.get("url"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                out.push(AgentModelContentPart::Text {
                    text: format!("[image:{}]", url),
                });
            }
            Some("tool_call") | Some("tool_use") => {
                let call_id = obj
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or(PromptSnapshotError::InvalidPart(
                        msg_index,
                        i,
                        "tool part missing `id`",
                    ))?
                    .to_string();
                let tool_id = obj
                    .get("tool_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let content = obj
                    .get("content")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                out.push(AgentModelContentPart::ToolResult {
                    call_id,
                    tool_id,
                    content,
                    is_error: obj
                        .get("is_error")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                });
            }
            _ => {
                return Err(PromptSnapshotError::InvalidPart(
                    msg_index,
                    i,
                    "unknown part type",
                ));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_marker_at_top_level() {
        let snapshot = json!({
            "_ferrite_agent_prompt_marker": "pending",
            "messages": []
        });
        assert_eq!(
            reject_unfinalized_snapshot(&snapshot),
            Err(PromptSnapshotError::UnfinalizedMarker(
                AGENT_PROMPT_MARKER_FIELD
            ))
        );
    }

    #[test]
    fn accepts_empty_marker_value_at_top_level() {
        let snapshot = json!({
            "_ferrite_agent_prompt_marker": "",
            "messages": []
        });
        assert!(reject_unfinalized_snapshot(&snapshot).is_ok());
    }

    #[test]
    fn accepts_legacy_marker_value() {
        let snapshot = json!({
            "_tauritavern_agent_prompt_marker": "pending",
            "messages": []
        });
        assert_eq!(
            reject_unfinalized_snapshot(&snapshot),
            Err(PromptSnapshotError::UnfinalizedMarker(
                LEGACY_TAURITAVERN_PROMPT_MARKER_FIELD
            ))
        );
    }

    #[test]
    fn rejects_marker_inside_message_content() {
        let snapshot = json!({
            "_ferrite_agent_prompt_marker": "",
            "messages": [
                { "role": "user", "content": "hello {{char}}" }
            ]
        });
        assert!(reject_unfinalized_snapshot(&snapshot).is_err());
    }

    #[test]
    fn detects_chat_completion_payload() {
        let snapshot = json!({
            "chatCompletionPayload": {
                "messages": [
                    { "role": "user", "content": "hi" }
                ]
            }
        });
        assert_eq!(
            snapshot_kind(&snapshot),
            AgentModelSnapshotKind::ChatCompletion
        );
        let msgs = messages_from_payload(&snapshot).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, AgentModelRole::User);
    }

    #[test]
    fn detects_frozen_run_input() {
        let snapshot = json!({
            "frozenRunInputSnapshot": { "messages": [] }
        });
        assert_eq!(
            snapshot_kind(&snapshot),
            AgentModelSnapshotKind::FrozenRunInput
        );
    }

    #[test]
    fn parses_tool_call_message() {
        let payload = json!({
            "messages": [
                {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [
                        {
                            "id": "call_123",
                            "type": "function",
                            "function": {
                                "name": "read_file",
                                "arguments": "{\"path\":\"/x\"}"
                            }
                        }
                    ]
                }
            ]
        });
        let msgs = messages_from_payload(&payload).unwrap();
        assert_eq!(msgs.len(), 1);
        match &msgs[0].parts[0] {
            AgentModelContentPart::ToolCall {
                call_id,
                model_alias,
                ..
            } => {
                assert_eq!(call_id, "call_123");
                assert_eq!(model_alias, "read_file");
            }
            _ => panic!("expected tool_call part"),
        }
    }

    #[test]
    fn parses_multimodal_content() {
        let payload = json!({
            "messages": [
                {
                    "role": "user",
                    "content": [
                        { "type": "text", "text": "look:" },
                        { "type": "image_url", "image_url": { "url": "https://x" } }
                    ]
                }
            ]
        });
        let msgs = messages_from_payload(&payload).unwrap();
        assert_eq!(msgs[0].parts.len(), 2);
    }

    #[test]
    fn context_policy_match_passes() {
        let snapshot = json!({
            "contextPolicy": {
                "initialChatHistoryMessages": -1,
                "includeActivatedWorldInfo": true
            }
        });
        let policy = AgentContextPolicy::default();
        assert!(validate_prompt_snapshot_context_policy(&snapshot, &policy).is_ok());
    }

    #[test]
    fn context_policy_mismatch_fails() {
        let snapshot = json!({
            "contextPolicy": {
                "initialChatHistoryMessages": 5,
                "includeActivatedWorldInfo": false
            }
        });
        let policy = AgentContextPolicy::default();
        assert_eq!(
            validate_prompt_snapshot_context_policy(&snapshot, &policy),
            Err(PromptSnapshotError::ContextPolicyMismatch)
        );
    }

    #[test]
    fn empty_payload_returns_empty_messages() {
        let payload = json!({});
        let msgs = messages_from_payload(&payload).unwrap();
        assert!(msgs.is_empty());
    }
}
