//! `GenerationType` 集成测试。
//!
//! 覆盖：serde snake_case 往返；旧 run.json（`"generationType":"chat"`）
//! 反序列化兼容；AgentRun 字段 round-trip。
//!
//! wire 兼容性依据：上游从未写过 `"chat"` 以外的值（loop_engine 硬编码），
//! snake_case 枚举把 `"chat"` 反序列化为 `Chat`，旧 run.json 无损读取。

use harness_core::{AgentChatRef, AgentRun, AgentRunPresentation, GenerationType};
use serde_json::json;

#[test]
fn generation_type_serde_snake_case_roundtrip() {
    // snake_case 命名往返：wire 值即 "chat" / "quiet" / "impersonate" / "continue"
    for (value, expected) in [
        (json!("chat"), GenerationType::Chat),
        (json!("quiet"), GenerationType::Quiet),
        (json!("impersonate"), GenerationType::Impersonate),
        (json!("continue"), GenerationType::Continue),
    ] {
        let parsed: GenerationType = serde_json::from_value(value.clone()).expect("deserialize");
        assert_eq!(parsed, expected);
        assert_eq!(serde_json::to_value(parsed).expect("serialize"), value);
    }
}

#[test]
fn generation_type_rejects_unknown_value() {
    // 未知值必须显式失败而不是静默映射，防止脏数据伪装成合法类型
    assert!(serde_json::from_value::<GenerationType>(json!("swipe")).is_err());
    assert!(serde_json::from_value::<GenerationType>(json!("")).is_err());
}

#[test]
fn legacy_run_json_with_string_chat_deserializes() {
    // 旧 run.json：generationType 是字符串 "chat"，skill_scope_refs 缺失走 default
    let legacy = json!({
        "id": "run_legacy",
        "workspaceId": "ws",
        "stableChatId": "chat",
        "chatRef": { "kind": "character", "characterId": "alice", "fileName": "alice" },
        "generationType": "chat",
        "inputMessageCount": 3,
        "presentation": "foreground",
        "status": "completed",
        "createdAt": "2026-09-05T00:00:00Z",
        "updatedAt": "2026-09-05T00:01:00Z"
    });
    let run: AgentRun = serde_json::from_value(legacy).expect("legacy run.json must load");
    assert_eq!(run.generation_type, GenerationType::Chat);
    assert_eq!(run.id, "run_legacy");
    assert!(matches!(run.chat_ref, AgentChatRef::Character { .. }));
}

#[test]
fn agent_run_generation_type_roundtrip_all_variants() {
    for expected in [
        GenerationType::Chat,
        GenerationType::Quiet,
        GenerationType::Impersonate,
        GenerationType::Continue,
    ] {
        let run = AgentRun {
            id: "run_x".into(),
            workspace_id: "ws".into(),
            stable_chat_id: "chat".into(),
            chat_ref: AgentChatRef::Group {
                chat_id: "g".into(),
            },
            generation_type: expected,
            profile_id: None,
            skill_scope_refs: Default::default(),
            persist_base_state_id: None,
            input_message_count: None,
            presentation: AgentRunPresentation::Foreground,
            status: harness_core::AgentRunStatus::Created,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let value = serde_json::to_value(&run).expect("serialize AgentRun");
        let parsed: AgentRun = serde_json::from_value(value).expect("deserialize AgentRun");
        assert_eq!(parsed.generation_type, expected);
    }
}
