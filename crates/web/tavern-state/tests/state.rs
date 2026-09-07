use tavern_state::Character;
use tavern_state::Message;
use tavern_state::{build_generate_body, build_system_prompt, resolve_swipe, seed_messages};

/// build_generate_body 展开 {{char}}/{{user}} 且带 _ferrite_agent_prompt_marker。
#[test]
fn test_build_generate_body_expands_variables_and_marker() {
    let character = Character {
        name: "Alice".to_string(),
        description: "A {{char}} desc".to_string(),
        personality: "friendly {{user}}".to_string(),
        scenario: String::new(),
        first_mes: String::new(),
        mes_example: String::new(),
        creator_notes: String::new(),
        tags: Vec::new(),
        extra: Default::default(),
    };
    let messages = vec![Message {
        name: "User".to_string(),
        is_user: true,
        send_date: String::new(),
        mes: "Hello {{char}}".to_string(),
        swipes: Vec::new(),
        swipe_id: None,
        extra: Default::default(),
    }];

    let body = build_generate_body(&character, "Alice", "Bob", &messages, "gpt-4");

    // 带 marker
    assert_eq!(
        body.get("_ferrite_agent_prompt_marker")
            .and_then(|v| v.as_str()),
        Some("")
    );

    // 展开 {{char}} / {{user}}
    let msgs = body.get("messages").unwrap().as_array().unwrap();
    let first_content = msgs[0].get("content").unwrap().as_str().unwrap();
    assert!(
        first_content.contains("Alice"),
        "system 应展开 {{char}}: {first_content}"
    );
    assert!(
        first_content.contains("Bob"),
        "system 应展开 {{user}}: {first_content}"
    );
    assert!(!first_content.contains("{{char}}"));
    assert!(!first_content.contains("{{user}}"));
}

/// 超长历史被 truncate 且首条 system 保留。
#[test]
fn test_truncate_preserves_system_and_trims_history() {
    let character = Character {
        name: "Alice".to_string(),
        description: "desc".to_string(),
        personality: String::new(),
        scenario: String::new(),
        first_mes: String::new(),
        mes_example: String::new(),
        creator_notes: String::new(),
        tags: Vec::new(),
        extra: Default::default(),
    };

    // 构造大量消息，每条 500 chars → ~125 tokens，4096 budget 只能保留约 32 条
    let mut messages = Vec::new();
    for i in 0..100 {
        messages.push(Message {
            name: "User".to_string(),
            is_user: i % 2 == 0,
            send_date: String::new(),
            mes: "x".repeat(500),
            swipes: Vec::new(),
            swipe_id: None,
            extra: Default::default(),
        });
    }

    let body = build_generate_body(&character, "Alice", "Bob", &messages, "gpt-4");
    let msgs = body.get("messages").unwrap().as_array().unwrap();

    // 首条是 system
    assert_eq!(msgs[0].get("role").unwrap().as_str().unwrap(), "system");

    // 总消息数应远小于 101（1 system + 100 history）
    assert!(msgs.len() < 50, "应被截断，实际: {}", msgs.len());
}

/// swipe_id 选中正确 swipe，越界回退 mes。
#[test]
fn test_resolve_swipe_selects_correct_swipe() {
    let msg = Message {
        name: "Alice".to_string(),
        is_user: false,
        send_date: String::new(),
        mes: "original".to_string(),
        swipes: vec!["swipe1".to_string(), "swipe2".to_string()],
        swipe_id: Some(1),
        extra: Default::default(),
    };
    assert_eq!(resolve_swipe(&msg), "swipe2");

    let msg_no_swipe = Message {
        name: "Alice".to_string(),
        is_user: false,
        send_date: String::new(),
        mes: "original".to_string(),
        swipes: vec!["swipe1".to_string()],
        swipe_id: None,
        extra: Default::default(),
    };
    assert_eq!(resolve_swipe(&msg_no_swipe), "swipe1");

    let msg_oob = Message {
        name: "Alice".to_string(),
        is_user: false,
        send_date: String::new(),
        mes: "fallback".to_string(),
        swipes: vec!["swipe1".to_string()],
        swipe_id: Some(5),
        extra: Default::default(),
    };
    assert_eq!(resolve_swipe(&msg_oob), "fallback");

    let msg_empty = Message {
        name: "Alice".to_string(),
        is_user: false,
        send_date: String::new(),
        mes: "fallback".to_string(),
        swipes: Vec::new(),
        swipe_id: Some(0),
        extra: Default::default(),
    };
    assert_eq!(resolve_swipe(&msg_empty), "fallback");
}

/// select_character 含 first_mes 时种子首条 assistant。
#[test]
fn test_seed_messages_with_first_mes() {
    let card = Character {
        name: "Alice".to_string(),
        description: String::new(),
        personality: String::new(),
        scenario: String::new(),
        first_mes: "Hello there!".to_string(),
        mes_example: String::new(),
        creator_notes: String::new(),
        tags: Vec::new(),
        extra: Default::default(),
    };
    let msgs = seed_messages(&card);
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].name, "Alice");
    assert!(!msgs[0].is_user);
    assert_eq!(msgs[0].mes, "Hello there!");
}

/// select_character 无 first_mes 时返回空。
#[test]
fn test_seed_messages_without_first_mes() {
    let card = Character {
        name: "Alice".to_string(),
        description: String::new(),
        personality: String::new(),
        scenario: String::new(),
        first_mes: String::new(),
        mes_example: String::new(),
        creator_notes: String::new(),
        tags: Vec::new(),
        extra: Default::default(),
    };
    let msgs = seed_messages(&card);
    assert!(msgs.is_empty());
}

/// build_system_prompt 拼接非空字段。
#[test]
fn test_build_system_prompt_joins_non_empty() {
    let character = Character {
        name: "Alice".to_string(),
        description: "desc".to_string(),
        personality: "friendly".to_string(),
        scenario: "forest".to_string(),
        first_mes: String::new(),
        mes_example: "hi".to_string(),
        creator_notes: String::new(),
        tags: Vec::new(),
        extra: Default::default(),
    };
    let prompt = build_system_prompt(&character);
    assert!(prompt.contains("desc"));
    assert!(prompt.contains("Personality: friendly"));
    assert!(prompt.contains("Scenario: forest"));
    assert!(prompt.contains("Example dialogue:\nhi"));
    assert!(prompt.contains("\n\n"));
}

/// build_system_prompt 全空时返回空字符串。
#[test]
fn test_build_system_prompt_all_empty() {
    let character = Character {
        name: "Alice".to_string(),
        description: String::new(),
        personality: String::new(),
        scenario: String::new(),
        first_mes: String::new(),
        mes_example: String::new(),
        creator_notes: String::new(),
        tags: Vec::new(),
        extra: Default::default(),
    };
    let prompt = build_system_prompt(&character);
    assert!(prompt.is_empty());
}
