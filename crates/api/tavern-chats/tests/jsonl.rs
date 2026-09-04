//! JSONL 保存语义、容错读取与最近列表。
//!
//! 覆盖老格式反序列化、新字段 round-trip、未知字段保留、system 消息行为。

use std::path::PathBuf;

use tavern_chats::{Message, MessageExtra, load, recent, save};

fn msg(mes: &str, is_user: bool) -> Message {
    Message {
        name: if is_user {
            "User".into()
        } else {
            "Char".into()
        },
        is_user,
        is_system: false,
        send_date: "2026-09-03".into(),
        mes: mes.into(),
        swipes: vec![],
        swipe_id: None,
        gen_started: None,
        gen_finished: None,
        title: None,
        force_avatar: None,
        extra: MessageExtra::default(),
        unknown: Default::default(),
    }
}

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ferrite-chats-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write_raw_line(dir: &std::path::Path, character: &str, chat: &str, line: serde_json::Value) {
    let char_dir = dir.join(character);
    std::fs::create_dir_all(&char_dir).unwrap();
    std::fs::write(
        char_dir.join(format!("{chat}.jsonl")),
        format!("{}\n", line),
    )
    .unwrap();
}

#[test]
fn roundtrip_preserves_order_and_flags() {
    let dir = tmp("roundtrip");
    save(&dir, "alice", "c1", &[msg("hi", true), msg("hello", false)]).unwrap();
    let back = load(&dir, "alice", "c1").unwrap();
    assert_eq!(back.len(), 2);
    assert_eq!(back[0].mes, "hi");
    assert!(back[0].is_user);
    assert!(!back[1].is_user);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn save_overwrites_rather_than_appends() {
    let dir = tmp("overwrite");
    save(&dir, "bob", "c1", &[msg("a", true), msg("b", false)]).unwrap();
    save(&dir, "bob", "c1", &[msg("a", true)]).unwrap();
    assert_eq!(load(&dir, "bob", "c1").unwrap().len(), 1);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn corrupt_line_is_skipped_not_fatal() {
    let dir = tmp("corrupt");
    save(&dir, "carol", "c1", &[msg("good", true)]).unwrap();
    let p = dir.join("carol").join("c1.jsonl");
    let mut raw = std::fs::read_to_string(&p).unwrap();
    raw.push_str("{not json\n");
    raw.push_str(&serde_json::to_string(&msg("after", false)).unwrap());
    raw.push('\n');
    std::fs::write(&p, raw).unwrap();
    let back = load(&dir, "carol", "c1").unwrap();
    assert_eq!(back.len(), 2);
    assert_eq!(back[1].mes, "after");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn unknown_fields_survive_roundtrip() {
    // 未知顶层字段透传到 `unknown`，反序列化后原样保留。
    let dir = tmp("unknown");
    let mut m = msg("x", false);
    m.unknown
        .insert("extensions".into(), serde_json::json!({"k": 1}));
    save(&dir, "dave", "c1", &[m]).unwrap();
    let back = load(&dir, "dave", "c1").unwrap();
    assert!(back[0].unknown.contains_key("extensions"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn traversal_in_names_is_rejected() {
    let dir = tmp("traversal");
    assert!(save(&dir, "..", "c1", &[]).is_err());
    assert!(load(&dir, "alice", "../../secrets").is_err());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn recent_lists_newest_first_with_preview() {
    let dir = tmp("recent");
    save(&dir, "eve", "old", &[msg("older msg", true)]).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    save(&dir, "eve", "new", &[msg("newer msg", true)]).unwrap();
    let r = recent(&dir, "eve").unwrap();
    assert_eq!(r[0].file_name, "new");
    assert_eq!(r[0].preview, "newer msg");
    std::fs::remove_dir_all(&dir).ok();
}

/// 审查给定的真实嵌套输入：顶层 `gen_started` / `force_avatar` + 嵌套 `extra`。
/// load→save 后元数据仍落在 nested `extra`；Rust 侧 `Message.extra` 能读出来。
#[test]
fn real_sillytavern_nested_extra_loads_and_reserializes() {
    let dir = tmp("real-nested");
    let raw_line = serde_json::json!({
        "name": "Char",
        "is_user": false,
        "send_date": "2026-01-01",
        "mes": "hi",
        "gen_started": "2026-01-01T00:00:00.000Z",
        "force_avatar": "/avatar.png",
        "extra": {
            "api": "openai",
            "model": "gpt-4",
            "reasoning": "think",
            "reasoning_duration": 123,
            "token_count": 42
        }
    });
    write_raw_line(&dir, "frank", "c1", raw_line);

    // 加载：Rust 侧字段正确填充
    let back = load(&dir, "frank", "c1").unwrap();
    assert_eq!(back.len(), 1);
    assert_eq!(
        back[0].gen_started.as_deref(),
        Some("2026-01-01T00:00:00.000Z")
    );
    assert_eq!(back[0].force_avatar.as_deref(), Some("/avatar.png"));
    assert_eq!(back[0].extra.api.as_deref(), Some("openai"));
    assert_eq!(back[0].extra.model.as_deref(), Some("gpt-4"));
    assert_eq!(back[0].extra.reasoning.as_deref(), Some("think"));
    assert_eq!(back[0].extra.reasoning_duration, Some(123));
    assert_eq!(back[0].extra.token_count, Some(42));

    // 落盘后结构仍是 nested `extra`，且 api/model/reasoning 落在 extra 内
    save(&dir, "frank", "c1", &back).unwrap();
    let raw = std::fs::read_to_string(dir.join("frank").join("c1.jsonl")).unwrap();
    let v: serde_json::Value = serde_json::from_str(raw.trim()).unwrap();
    let extra = v.get("extra").expect("extra must remain a nested object");
    assert_eq!(extra["api"], "openai");
    assert_eq!(extra["model"], "gpt-4");
    assert_eq!(extra["reasoning"], "think");
    assert_eq!(extra["reasoning_duration"], 123);
    assert_eq!(extra["token_count"], 42);
    // 顶层不应冒出 api/model/reasoning
    assert!(v.get("api").is_none());
    assert!(v.get("model").is_none());
    assert!(v.get("reasoning").is_none());
    // 顶层保留
    assert_eq!(v["gen_started"], "2026-01-01T00:00:00.000Z");
    assert_eq!(v["force_avatar"], "/avatar.png");
    std::fs::remove_dir_all(&dir).ok();
}

/// Rust 构造 → save：api/model/reasoning 落在 nested extra，不冒到顶层。
#[test]
fn new_message_writes_extra_as_nested_object() {
    let dir = tmp("write-nested");
    let mut m = msg("hi", false);
    m.extra.api = Some("openai".into());
    m.extra.model = Some("gpt-4".into());
    m.extra.reasoning = Some("think".into());
    m.extra.reasoning_duration = Some(123);
    m.extra.token_count = Some(42);
    save(&dir, "grace", "c1", &[m]).unwrap();

    let raw = std::fs::read_to_string(dir.join("grace").join("c1.jsonl")).unwrap();
    let v: serde_json::Value = serde_json::from_str(raw.trim()).unwrap();
    let extra = v.get("extra").expect("extra must be a nested object");
    assert_eq!(extra["api"], "openai");
    assert_eq!(extra["model"], "gpt-4");
    assert_eq!(extra["reasoning"], "think");
    assert_eq!(extra["reasoning_duration"], 123);
    assert_eq!(extra["token_count"], 42);
    // 顶层不出现
    assert!(v.get("api").is_none());
    assert!(v.get("model").is_none());
    assert!(v.get("reasoning").is_none());
    std::fs::remove_dir_all(&dir).ok();
}

/// 老格式 JSONL（无 nested extra、无新顶层字段）能正常加载。
#[test]
fn legacy_jsonl_without_new_fields_loads() {
    let dir = tmp("legacy");
    write_raw_line(
        &dir,
        "heidi",
        "legacy",
        serde_json::json!({
            "name": "Char",
            "is_user": false,
            "send_date": "2026-01-01",
            "mes": "old hello",
            "swipes": ["v1", "v2"],
            "swipe_id": 1,
        }),
    );
    let back = load(&dir, "heidi", "legacy").unwrap();
    assert_eq!(back.len(), 1);
    assert!(!back[0].is_system, "is_system should default to false");
    assert_eq!(back[0].mes, "old hello");
    assert_eq!(back[0].swipes, vec!["v1", "v2"]);
    assert_eq!(back[0].swipe_id, Some(1));
    assert!(back[0].extra.api.is_none());
    assert!(back[0].extra.model.is_none());
    assert!(back[0].gen_started.is_none());
    assert!(back[0].force_avatar.is_none());
    std::fs::remove_dir_all(&dir).ok();
}

/// `is_system` round-trip：序列化进 JSON 并能读回。
#[test]
fn system_message_round_trips() {
    let dir = tmp("system");
    let mut m = msg("[SYSTEM]: be terse", false);
    m.is_system = true;
    save(&dir, "ivan", "c1", &[m]).unwrap();

    let raw = std::fs::read_to_string(dir.join("ivan").join("c1.jsonl")).unwrap();
    let v: serde_json::Value = serde_json::from_str(raw.trim()).unwrap();
    assert_eq!(v["is_system"], true);

    let back = load(&dir, "ivan", "c1").unwrap();
    assert!(back[0].is_system);
    std::fs::remove_dir_all(&dir).ok();
}

/// JSONL 缺 `is_system`：反序列化为 false（避免老数据被误判为 system）。
#[test]
fn is_system_defaults_to_false_when_missing() {
    let dir = tmp("system-default");
    write_raw_line(
        &dir,
        "judy",
        "c1",
        serde_json::json!({
            "name": "Char",
            "is_user": false,
            "send_date": "2026-01-01",
            "mes": "hi",
        }),
    );
    let back = load(&dir, "judy", "c1").unwrap();
    assert!(!back[0].is_system);
    std::fs::remove_dir_all(&dir).ok();
}

/// 未知顶层字段落入 `Message.unknown`。
#[test]
fn unknown_top_level_field_lands_in_unknown() {
    let dir = tmp("unknown-top");
    write_raw_line(
        &dir,
        "kim",
        "c1",
        serde_json::json!({
            "name": "Char",
            "is_user": false,
            "send_date": "2026-01-01",
            "mes": "hi",
            "custom_marker": "abc",
        }),
    );
    let back = load(&dir, "kim", "c1").unwrap();
    assert_eq!(
        back[0].unknown.get("custom_marker"),
        Some(&serde_json::json!("abc"))
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// 未知 extra 内部字段落入 `MessageExtra.additional`。
#[test]
fn unknown_extra_field_lands_in_additional() {
    let dir = tmp("unknown-extra");
    write_raw_line(
        &dir,
        "leo",
        "c1",
        serde_json::json!({
            "name": "Char",
            "is_user": false,
            "send_date": "2026-01-01",
            "mes": "hi",
            "extra": {
                "api": "openai",
                "vendor_tag": "abc"
            }
        }),
    );
    let back = load(&dir, "leo", "c1").unwrap();
    assert_eq!(back[0].extra.api.as_deref(), Some("openai"));
    assert_eq!(
        back[0].extra.additional.get("vendor_tag"),
        Some(&serde_json::json!("abc"))
    );
    std::fs::remove_dir_all(&dir).ok();
}
