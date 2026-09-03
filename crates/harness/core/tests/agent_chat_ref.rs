//! `AgentChatRef` 的契约测试 — tagged enum ABI 形状 + 字段拒绝。
//!
//! 整段搬运自原 `tt-domain/src/models/agent/mod.rs` 中的单测，确保
//! `Character` / `Group` 两种变体的 camelCase JSON 形态可被前端正常解析，
//! 且 `deny_unknown_fields` 会拦截内部 snake_case 字段名（防止 ABI 退化）。

use harness_core::run::AgentChatRef;

#[test]
fn chat_ref_accepts_frontend_abi_shape() {
    let ref_from_frontend: AgentChatRef = serde_json::from_value(serde_json::json!({
        "kind": "character",
        "characterId": "Seraphina",
        "fileName": "chapter-1"
    }))
    .expect("frontend character ref");

    assert_eq!(
        ref_from_frontend,
        AgentChatRef::Character {
            character_id: "Seraphina".to_string(),
            file_name: "chapter-1".to_string(),
        }
    );

    let group_ref: AgentChatRef = serde_json::from_value(serde_json::json!({
        "kind": "group",
        "chatId": "group-chat"
    }))
    .expect("frontend group ref");

    assert_eq!(
        group_ref,
        AgentChatRef::Group {
            chat_id: "group-chat".to_string(),
        }
    );
}

#[test]
fn chat_ref_serializes_to_frontend_abi_shape() {
    let value = serde_json::to_value(AgentChatRef::Character {
        character_id: "Seraphina".to_string(),
        file_name: "chapter-1".to_string(),
    })
    .expect("serialize character ref");

    assert_eq!(
        value,
        serde_json::json!({
            "kind": "character",
            "characterId": "Seraphina",
            "fileName": "chapter-1"
        })
    );
}

#[test]
fn chat_ref_rejects_internal_field_names_at_abi_boundary() {
    let result = serde_json::from_value::<AgentChatRef>(serde_json::json!({
        "kind": "character",
        "characterId": "Seraphina",
        "fileName": "chapter-1",
        "character_id": "Seraphina",
        "file_name": "chapter-1"
    }));

    assert!(
        result.is_err(),
        "snake_case 字段名必须被拒绝 — 这就是 ABI 屏障的目的"
    );
}

#[test]
fn chat_ref_rejects_missing_required_camel_case_fields() {
    let result = serde_json::from_value::<AgentChatRef>(serde_json::json!({
        "kind": "character",
        "character_id": "Seraphina",
        "file_name": "chapter-1"
    }));

    assert!(
        result.is_err(),
        "缺少完整 camelCase 必填字段的输入必须被拒绝"
    );
}

#[test]
fn chat_ref_rejects_unknown_kind_tag() {
    let result = serde_json::from_value::<AgentChatRef>(serde_json::json!({
        "kind": "mystery",
        "characterId": "Seraphina",
        "fileName": "chapter-1"
    }));

    assert!(
        result.is_err(),
        "未知 `kind` 标签必须被拒绝，避免静默落入错误变体"
    );
}

#[test]
fn chat_ref_round_trip_through_camel_case() {
    let original = AgentChatRef::Character {
        character_id: "Seraphina".to_string(),
        file_name: "chapter-1".to_string(),
    };
    let serialized = serde_json::to_string(&original).expect("serialize");
    let round_tripped: AgentChatRef = serde_json::from_str(&serialized).expect("round trip");
    assert_eq!(round_tripped, original);
}
