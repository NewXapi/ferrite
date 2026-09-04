//! `prompt_snapshot` 集成测试。
//!
//! 通过公共 API（`messages_from_payload` / `reject_unfinalized_snapshot` /
//! `snapshot_kind` / `validate_prompt_snapshot_context_policy`）覆盖 marker 校验、
//! kind 识别、role/content 解析、tool call/result 边界。
//!
//! ponytail: 单元测试全部下沉到 tests/ —— Cargo 对 tests/ 目录默认只在
//! `cargo test` 编译，写 `#[cfg(test)]` 在这里反而是 boilerplate 错误。

use harness_core::AgentContextPolicy;
use harness_prompt::{
    AGENT_PROMPT_MARKER_FIELD, AgentModelContentPart, AgentModelMessage, AgentModelRole,
    AgentModelSnapshotKind, LEGACY_TAURITAVERN_PROMPT_MARKER_FIELD, PromptSnapshotError,
    messages_from_payload, reject_unfinalized_snapshot, snapshot_kind,
    validate_prompt_snapshot_context_policy,
};
use serde_json::{Value, json};

// ===== marker 校验 =====

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
fn rejects_marker_inside_nested_chat_completion_payload() {
    // marker check 必须复用 locate_messages，否则 nested payload 内的
    // 未物化 marker 会被漏检。
    let snapshot = json!({
        "chatCompletionPayload": {
            "messages": [
                { "role": "user", "content": "hello {{user}}" }
            ]
        }
    });
    assert_eq!(
        reject_unfinalized_snapshot(&snapshot),
        Err(PromptSnapshotError::UnfinalizedMarker(
            AGENT_PROMPT_MARKER_FIELD
        ))
    );
}

#[test]
fn accepts_nested_frozen_run_input_snapshot() {
    // frozenRunInputSnapshot 也用 locate_messages 定位；它 kind 声称支持
    let snapshot = json!({
        "frozenRunInputSnapshot": {
            "messages": [
                { "role": "user", "content": "hi" }
            ]
        }
    });
    reject_unfinalized_snapshot(&snapshot).expect("must accept finalized frozen snapshot");
    assert_eq!(
        snapshot_kind(&snapshot),
        AgentModelSnapshotKind::FrozenRunInput
    );
    let msgs = messages_from_payload(&snapshot).unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].role, AgentModelRole::User);
}

#[test]
fn rejects_marker_inside_frozen_run_input_snapshot() {
    let snapshot = json!({
        "frozenRunInputSnapshot": {
            "messages": [
                { "role": "user", "content": "hello {{user}}" }
            ]
        }
    });
    assert_eq!(
        reject_unfinalized_snapshot(&snapshot),
        Err(PromptSnapshotError::UnfinalizedMarker(
            AGENT_PROMPT_MARKER_FIELD
        ))
    );
}

// ===== kind 识别 =====

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

// ===== role / content 解析 =====

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
fn empty_payload_returns_empty_messages() {
    let payload = json!({});
    let msgs = messages_from_payload(&payload).unwrap();
    assert!(msgs.is_empty());
}

// ===== tool role wire contract（review blockers） =====

#[test]
fn tool_role_string_content_becomes_tool_result_with_call_id() {
    // role=tool + 字符串 content 必须保留 tool_call_id 并产出 ToolResult；
    // 不能静默降级为 Text（runtime 需要 call_id 关联前置 tool_call）。
    let payload = json!({
        "messages": [
            { "role": "user", "content": "ask" },
            {
                "role": "assistant", "content": null,
                "tool_calls": [{
                    "id": "call_xyz", "type": "function",
                    "function": { "name": "read_file", "arguments": "{}" }
                }]
            },
            { "role": "tool", "tool_call_id": "call_xyz", "content": "file contents" }
        ]
    });
    let msgs = messages_from_payload(&payload).unwrap();
    assert_eq!(msgs.len(), 3);
    match &msgs[2].parts[0] {
        AgentModelContentPart::ToolResult {
            call_id, content, ..
        } => {
            assert_eq!(call_id, "call_xyz");
            assert_eq!(content, "file contents");
        }
        _ => panic!("role=tool must produce ToolResult, not Text"),
    }
    // 关联性：assistant(tool_call) 与 tool(result) 共享 call_id
    if let AgentModelContentPart::ToolCall { call_id: cc, .. } = &msgs[1].parts[0] {
        assert_eq!(cc, "call_xyz");
    } else {
        panic!("expected tool_call part");
    }
}

#[test]
fn tool_role_missing_tool_call_id_errors() {
    let payload = json!({
        "messages": [
            { "role": "tool", "content": "file contents" }
        ]
    });
    let err = messages_from_payload(&payload).unwrap_err();
    assert_eq!(err, PromptSnapshotError::MissingToolCallId(0));
}

#[test]
fn assistant_null_content_with_tool_calls_produces_single_tool_call() {
    // 修复前：content=null 分支先 push 一遍 tool_calls，line 324 又 push 一遍 → 重复
    let payload = json!({
        "messages": [
            {
                "role": "assistant",
                "content": null,
                "tool_calls": [
                    {
                        "id": "call_dup",
                        "type": "function",
                        "function": { "name": "read_file", "arguments": "{}" }
                    }
                ]
            }
        ]
    });
    let msgs = messages_from_payload(&payload).unwrap();
    assert_eq!(msgs.len(), 1);
    // 严格一份 ToolCall
    let tool_count = msgs[0]
        .parts
        .iter()
        .filter(|p| matches!(p, AgentModelContentPart::ToolCall { .. }))
        .count();
    assert_eq!(
        tool_count, 1,
        "tool_calls must produce exactly one ToolCall part"
    );
    // 直接用 serde 反序列化 wire JSON（不走 OpenAI 解析）—— 结果必须稳定（无重复累积）
    let serialized = serde_json::to_value(&msgs[0]).unwrap();
    let msgs2: AgentModelMessage = serde_json::from_value(serialized).unwrap();
    let tool_count2 = msgs2
        .parts
        .iter()
        .filter(|p| matches!(p, AgentModelContentPart::ToolCall { .. }))
        .count();
    assert_eq!(tool_count2, 1, "roundtrip must keep exactly one ToolCall");
}

#[test]
fn tool_use_part_parses_as_tool_call_with_arguments() {
    // Anthropic 风格 tool_use → ToolCall（不是 ToolResult）
    let payload = json!({
        "messages": [
            {
                "role": "assistant",
                "content": [
                    {
                        "type": "tool_use",
                        "id": "toolu_1",
                        "name": "read_file",
                        "input": { "path": "/tmp/x" }
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
            arguments,
            ..
        } => {
            assert_eq!(call_id, "toolu_1");
            assert_eq!(model_alias, "read_file");
            assert_eq!(
                arguments.get("path").and_then(|v| v.as_str()),
                Some("/tmp/x")
            );
        }
        other => panic!("expected ToolCall, got {:?}", other),
    }
}

#[test]
fn tool_result_part_with_tool_use_id_parses_as_tool_result() {
    // Anthropic 风格 tool_result → ToolResult
    let payload = json!({
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "toolu_1",
                        "content": "file contents",
                        "is_error": false
                    }
                ]
            }
        ]
    });
    let msgs = messages_from_payload(&payload).unwrap();
    assert_eq!(msgs.len(), 1);
    match &msgs[0].parts[0] {
        AgentModelContentPart::ToolResult {
            call_id, content, ..
        } => {
            assert_eq!(call_id, "toolu_1");
            assert_eq!(content, "file contents");
        }
        other => panic!("expected ToolResult, got {:?}", other),
    }
}

// ===== camelCase wire JSON（review blocker） =====

#[test]
fn content_parts_serialize_to_camelcase_wire_json() {
    // AgentModelContentPart 字段必须 camelCase，与 AgentModelMessage 等 ABI 一致。
    // 真实 JSON 序列化断言：tool_call 与 tool_result 都要看到 camelCase 字段名。
    let tool_call = AgentModelContentPart::ToolCall {
        call_id: "c1".into(),
        tool_id: "builtin:read".into(),
        model_alias: "read_file".into(),
        arguments: json!({ "path": "/x" }),
    };
    let tool_result = AgentModelContentPart::ToolResult {
        call_id: "c1".into(),
        tool_id: "builtin:read".into(),
        content: "ok".into(),
        is_error: false,
    };

    // tool_call
    let v: Value = serde_json::to_value(&tool_call).unwrap();
    assert_eq!(v["type"], "tool_call");
    assert_eq!(v["callId"], "c1");
    assert_eq!(v["toolId"], "builtin:read");
    assert_eq!(v["modelAlias"], "read_file");
    assert!(
        v.get("call_id").is_none(),
        "snake_case call_id must NOT appear"
    );
    assert!(
        v.get("tool_id").is_none(),
        "snake_case tool_id must NOT appear"
    );
    assert!(
        v.get("model_alias").is_none(),
        "snake_case model_alias must NOT appear"
    );

    // tool_result
    let v: Value = serde_json::to_value(&tool_result).unwrap();
    assert_eq!(v["type"], "tool_result");
    assert_eq!(v["callId"], "c1");
    assert_eq!(v["toolId"], "builtin:read");
    assert_eq!(v["content"], "ok");
    assert_eq!(v["isError"], false);
    assert!(v.get("call_id").is_none());
    assert!(v.get("tool_id").is_none());
    assert!(v.get("is_error").is_none());

    // 反向：从 camelCase JSON 反序列化也能拿到对应字段
    let json = json!({
        "type": "tool_call",
        "callId": "c2",
        "toolId": "builtin:read",
        "modelAlias": "read_file",
        "arguments": { "path": "/y" }
    });
    let part: AgentModelContentPart = serde_json::from_value(json).unwrap();
    if let AgentModelContentPart::ToolCall {
        call_id,
        tool_id,
        model_alias,
        arguments,
    } = part
    {
        assert_eq!(call_id, "c2");
        assert_eq!(tool_id, "builtin:read");
        assert_eq!(model_alias, "read_file");
        assert_eq!(arguments["path"], "/y");
    } else {
        panic!("expected ToolCall");
    }
}

// ===== context policy =====

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
fn context_policy_validation() {
    let policy = AgentContextPolicy::default();
    // 空 policy override → 默认通过
    let snapshot = json!({"messages": []});
    assert!(validate_prompt_snapshot_context_policy(&snapshot, &policy).is_ok());

    // override 与 profile 一致 → 通过
    let snapshot_match = json!({
        "contextPolicy": {
            "initialChatHistoryMessages": -1,
            "includeActivatedWorldInfo": true
        }
    });
    assert!(validate_prompt_snapshot_context_policy(&snapshot_match, &policy).is_ok());

    // override 与 profile 不一致 → 失败
    let snapshot_mismatch = json!({
        "contextPolicy": {
            "initialChatHistoryMessages": 5,
            "includeActivatedWorldInfo": false
        }
    });
    assert_eq!(
        validate_prompt_snapshot_context_policy(&snapshot_mismatch, &policy),
        Err(PromptSnapshotError::ContextPolicyMismatch)
    );
}

// ===== legacy tauritavern marker =====

#[test]
fn parses_legacy_tauritavern_marker() {
    let snapshot: Value = json!({
        "_tauritavern_agent_prompt_marker": "pending",
        "messages": []
    });
    assert_eq!(
        reject_unfinalized_snapshot(&snapshot),
        Err(PromptSnapshotError::UnfinalizedMarker(
            "_tauritavern_agent_prompt_marker"
        ))
    );
}

// ===== full round trip from frontend snapshot =====

#[test]
fn full_round_trip_from_frontend_snapshot() {
    let snapshot = json!({
        AGENT_PROMPT_MARKER_FIELD: "",
        "messages": [
            { "role": "system", "content": "You are a helpful assistant." },
            { "role": "user", "content": "hi" },
            {
                "role": "assistant",
                "content": null,
                "tool_calls": [
                    {
                        "id": "call_abc",
                        "type": "function",
                        "function": {
                            "name": "read_file",
                            "arguments": "{\"path\":\"/tmp/x\"}"
                        }
                    }
                ]
            },
            {
                "role": "tool",
                "tool_call_id": "call_abc",
                "content": "file contents"
            }
        ]
    });

    // 1. 拒绝未物化 → 通过
    reject_unfinalized_snapshot(&snapshot).expect("must accept finalized snapshot");

    // 2. 识别 kind
    assert_eq!(snapshot_kind(&snapshot), AgentModelSnapshotKind::Other);

    // 3. 解析 messages
    let msgs = messages_from_payload(&snapshot).expect("messages");
    assert_eq!(msgs.len(), 4);
    assert_eq!(msgs[0].role, AgentModelRole::System);
    assert_eq!(msgs[3].role, AgentModelRole::Tool);
    assert!(matches!(
        msgs[2].parts[0],
        AgentModelContentPart::ToolCall { .. }
    ));
    // tool 消息必须保留 call_id 关联
    if let AgentModelContentPart::ToolResult { call_id, .. } = &msgs[3].parts[0] {
        assert_eq!(call_id, "call_abc");
    } else {
        panic!("tool role must produce ToolResult");
    }
}

#[test]
fn rejects_snapshot_with_unresolved_macros() {
    let snapshot = json!({
        AGENT_PROMPT_MARKER_FIELD: "",
        "messages": [
            { "role": "user", "content": "hi {{user}}" }
        ]
    });

    let result = reject_unfinalized_snapshot(&snapshot);
    assert!(matches!(
        result,
        Err(PromptSnapshotError::UnfinalizedMarker(_))
    ));
}

#[test]
fn rejects_snapshot_with_pending_top_level_marker() {
    let snapshot = json!({
        AGENT_PROMPT_MARKER_FIELD: "pending",
        "messages": []
    });

    let result = reject_unfinalized_snapshot(&snapshot);
    assert_eq!(
        result,
        Err(PromptSnapshotError::UnfinalizedMarker(
            AGENT_PROMPT_MARKER_FIELD
        ))
    );
}

#[test]
fn detects_nested_chat_completion_payload() {
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
    let msgs = messages_from_payload(&snapshot).expect("messages");
    assert_eq!(msgs.len(), 1);
}

#[test]
fn detects_frozen_run_input_snapshot_kind_and_parse() {
    let snapshot = json!({
        "frozenRunInputSnapshot": {
            "messages": [{ "role": "user", "content": "x" }]
        }
    });
    assert_eq!(
        snapshot_kind(&snapshot),
        AgentModelSnapshotKind::FrozenRunInput
    );
    let msgs = messages_from_payload(&snapshot).expect("messages");
    assert_eq!(msgs.len(), 1);
}
