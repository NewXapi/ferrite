//! 路径安全、原子写与目录布局。

use std::path::Path;

use tavern_storage::{DataRoot, is_under, join_checked, sanitize_name, write_atomic};

#[test]
fn rejects_traversal_and_separators() {
    assert!(sanitize_name("../etc/passwd").is_err());
    assert!(sanitize_name("a/b").is_err());
    assert!(sanitize_name("..").is_err());
    assert!(sanitize_name("").is_err());
    assert!(sanitize_name("normal name.png").is_ok());
}

#[test]
fn join_checked_blocks_escape() {
    let parent = Path::new("/data/u/chats");
    assert!(join_checked(parent, "../secrets.json").is_err());
    assert_eq!(
        join_checked(parent, "chat.jsonl").unwrap(),
        parent.join("chat.jsonl")
    );
}

#[test]
fn is_under_rejects_parent_refs() {
    let parent = Path::new("/data/u/chats");
    assert!(is_under(parent, &parent.join("a.jsonl")));
    assert!(!is_under(parent, Path::new("/data/u/secrets.json")));
    assert!(!is_under(parent, &parent.join("../secrets.json")));
}

#[test]
fn atomic_write_replaces_content() {
    let dir = std::env::temp_dir().join(format!("ferrite-st-{}-atomic", std::process::id()));
    let f = dir.join("x.json");
    write_atomic(&f, b"first").unwrap();
    write_atomic(&f, b"second").unwrap();
    assert_eq!(std::fs::read(&f).unwrap(), b"second");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn user_dirs_layout() {
    let root = DataRoot::new("/data");
    let u = root.user("default-user");
    assert_eq!(u.characters(), Path::new("/data/default-user/characters"));
    assert_eq!(
        u.settings_file(),
        Path::new("/data/default-user/settings.json")
    );
}
