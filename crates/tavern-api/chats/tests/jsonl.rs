//! JSONL 保存语义、容错读取与最近列表。

use std::path::PathBuf;

use tavern_chats::{Message, load, recent, save};

fn msg(mes: &str, is_user: bool) -> Message {
    Message {
        name: if is_user { "User".into() } else { "Char".into() },
        is_user,
        send_date: "2026-09-03".into(),
        mes: mes.into(),
        swipes: vec![],
        swipe_id: None,
        extra: Default::default(),
    }
}

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ferrite-chats-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
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
    m.extra.insert("extensions".into(), serde_json::json!({"k": 1}));
    save(&dir, "dave", "c1", &[m]).unwrap();
    let back = load(&dir, "dave", "c1").unwrap();
    assert!(back[0].extra.contains_key("extensions"));
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
