//! Snapshot marker + payload 解析集成测试。
//!
//! 验证从 OpenAI-compatible JSON 解析成 `AgentModelMessage` 的边界行为。

use harness_core::AgentContextPolicy;
use harness_prompt::{
    AGENT_PROMPT_MARKER_FIELD, AgentModelContentPart, AgentModelRole, AgentModelSnapshotKind,
    PromptSnapshotError, messages_from_payload, reject_unfinalized_snapshot, snapshot_kind,
    validate_prompt_snapshot_context_policy,
};
use serde_json::{Value, json};

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
fn detects_frozen_run_input_snapshot() {
    let snapshot = json!({
        "frozenRunInputSnapshot": {
            "messages": [{ "role": "user", "content": "x" }]
        }
    });
    assert_eq!(
        snapshot_kind(&snapshot),
        AgentModelSnapshotKind::FrozenRunInput
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
