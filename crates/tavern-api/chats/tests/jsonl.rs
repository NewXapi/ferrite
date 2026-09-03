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
        extra: MessageExtra::default(),
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
    let dir = tmp("unknown");
    let mut m = msg("x", false);
    m.extra
        .additional
        .insert("extensions".into(), serde_json::json!({"k": 1}));
    save(&dir, "dave", "c1", &[m]).unwrap();
    let back = load(&dir, "dave", "c1").unwrap();
    assert!(back[0].extra.additional.contains_key("extensions"));
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

#[test]
fn legacy_jsonl_without_new_fields_loads() {
    // 老格式 JSONL：没有 is_system / MessageExtra 字段，顶层 swipes/swipe_id。
    // 反序列化应填默认值而非失败。
    let dir = tmp("legacy");
    write_raw_line(
        &dir,
        "frank",
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
    let back = load(&dir, "frank", "legacy").unwrap();
    assert_eq!(back.len(), 1);
    assert!(!back[0].is_system, "is_system should default to false");
    assert_eq!(back[0].mes, "old hello");
    assert_eq!(back[0].swipes, vec!["v1", "v2"]);
    assert_eq!(back[0].swipe_id, Some(1));
    assert!(back[0].extra.api.is_none());
    assert!(back[0].extra.model.is_none());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn message_extra_round_trips_at_top_level() {
    // MessageExtra 的字段在 JSON 中应保持顶层（SillyTavern 对齐）。
    let dir = tmp("extra-roundtrip");
    let mut m = msg("hi", false);
    m.extra.api = Some("openai".into());
    m.extra.model = Some("gpt-4".into());
    m.extra.reasoning = Some("thinking...".into());
    m.extra.reasoning_duration = Some(123);
    m.extra.token_count = Some(42);
    save(&dir, "grace", "c1", &[m]).unwrap();

    let raw = std::fs::read_to_string(dir.join("grace").join("c1.jsonl")).unwrap();
    let v: serde_json::Value = serde_json::from_str(raw.trim()).unwrap();
    // 已知字段在顶层
    assert_eq!(v["api"], "openai");
    assert_eq!(v["model"], "gpt-4");
    assert_eq!(v["reasoning"], "thinking...");
    assert_eq!(v["reasoning_duration"], 123);
    assert_eq!(v["token_count"], 42);
    // 没有 nested `extra` 包装
    assert!(v.get("extra").is_none());

    let back = load(&dir, "grace", "c1").unwrap();
    assert_eq!(back[0].extra.api.as_deref(), Some("openai"));
    assert_eq!(back[0].extra.model.as_deref(), Some("gpt-4"));
    assert_eq!(back[0].extra.reasoning.as_deref(), Some("thinking..."));
    assert_eq!(back[0].extra.reasoning_duration, Some(123));
    assert_eq!(back[0].extra.token_count, Some(42));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn system_message_round_trips() {
    // is_system 应当序列化进 JSON 并能被读回。
    let dir = tmp("system");
    let mut m = msg("[SYSTEM]: be terse", false);
    m.is_system = true;
    save(&dir, "heidi", "c1", &[m]).unwrap();

    let raw = std::fs::read_to_string(dir.join("heidi").join("c1.jsonl")).unwrap();
    let v: serde_json::Value = serde_json::from_str(raw.trim()).unwrap();
    assert_eq!(v["is_system"], true);

    let back = load(&dir, "heidi", "c1").unwrap();
    assert!(back[0].is_system);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn is_system_defaults_to_false_when_missing() {
    // JSONL 里不写 is_system：反序列化得 false，不是 true（避免老数据被误判）。
    let dir = tmp("system-default");
    write_raw_line(
        &dir,
        "ivan",
        "c1",
        serde_json::json!({
            "name": "Char",
            "is_user": false,
            "send_date": "2026-01-01",
            "mes": "hi",
        }),
    );
    let back = load(&dir, "ivan", "c1").unwrap();
    assert!(!back[0].is_system);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn unknown_top_level_field_lands_in_extra_additional() {
    // 老 JSONL 顶层出现自定义字段，不能因为后端没声明就丢。
    // 它要么走 MessageExtra 的具名字段，要么进 additional。
    let dir = tmp("unknown-top");
    write_raw_line(
        &dir,
        "judy",
        "c1",
        serde_json::json!({
            "name": "Char",
            "is_user": false,
            "send_date": "2026-01-01",
            "mes": "hi",
            "custom_marker": "abc",
        }),
    );
    let back = load(&dir, "judy", "c1").unwrap();
    assert_eq!(
        back[0].extra.additional.get("custom_marker"),
        Some(&serde_json::json!("abc"))
    );
    std::fs::remove_dir_all(&dir).ok();
}
