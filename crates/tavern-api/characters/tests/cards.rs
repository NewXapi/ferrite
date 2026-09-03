//! 角色卡 CRUD 与列表容错。

use std::path::PathBuf;

use tavern_characters::{Character, delete, get, list, png, save};

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ferrite-chars-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn card(name: &str) -> Character {
    Character { name: name.into(), description: "desc".into(), ..Default::default() }
}

#[test]
fn save_then_get_roundtrips() {
    let dir = tmp("roundtrip");
    save(&dir, "alice", &card("Alice"), Some(&png::minimal_png())).unwrap();
    let back = get(&dir, "alice").unwrap();
    assert_eq!(back.name, "Alice");
    assert_eq!(back.description, "desc");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn edit_without_new_avatar_keeps_working() {
    let dir = tmp("edit");
    save(&dir, "bob", &card("Bob"), Some(&png::minimal_png())).unwrap();
    let mut c = get(&dir, "bob").unwrap();
    c.personality = "calm".into();
    save(&dir, "bob", &c, None).unwrap();
    assert_eq!(get(&dir, "bob").unwrap().personality, "calm");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn list_skips_unparseable_files() {
    let dir = tmp("list");
    save(&dir, "carol", &card("Carol"), Some(&png::minimal_png())).unwrap();
    std::fs::write(dir.join("junk.png"), b"not a png").unwrap();
    let items = list(&dir).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "Carol");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn traversal_rejected() {
    let dir = tmp("traversal");
    assert!(get(&dir, "../secrets").is_err());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn delete_removes_card() {
    let dir = tmp("delete");
    save(&dir, "dan", &card("Dan"), Some(&png::minimal_png())).unwrap();
    delete(&dir, "dan").unwrap();
    assert!(get(&dir, "dan").is_err());
    std::fs::remove_dir_all(&dir).ok();
}
