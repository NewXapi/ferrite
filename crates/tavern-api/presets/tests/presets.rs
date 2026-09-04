//! presets 文件 CRUD、路径安全与未知 apiId。

use std::path::PathBuf;

use serde_json::json;
use tavern_presets::{delete, list, load, restore, save};

fn root(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ferrite-presets-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn folder_for_known_and_unknown() {
    assert_eq!(
        tavern_presets::folder_for("openai"),
        Some("OpenAI Settings")
    );
    assert_eq!(tavern_presets::folder_for("instruct"), Some("instruct"));
    assert_eq!(tavern_presets::folder_for("context"), Some("context"));
    assert_eq!(tavern_presets::folder_for("sysprompt"), Some("sysprompt"));
    assert_eq!(tavern_presets::folder_for("reasoning"), Some("reasoning"));
    #[rustfmt::skip]
    const KOBOLD: &str = "KoboldAI Settings";
    #[rustfmt::skip]
    const NOVEL: &str = "NovelAI Settings";
    #[rustfmt::skip]
    const TGWEBUI: &str = "TextGen Settings";
    assert_eq!(tavern_presets::folder_for("kobold"), Some(KOBOLD));
    assert_eq!(tavern_presets::folder_for("novel"), Some(NOVEL));
    assert_eq!(
        tavern_presets::folder_for("textgenerationwebui"),
        Some(TGWEBUI)
    );
    assert_eq!(tavern_presets::folder_for("nope"), None);
}

#[test]
fn save_then_load_roundtrips() {
    let r = root("roundtrip");
    let preset = json!({ "temperature": 0.7, "top_p": 0.9, "messages": [{"role": "system"}] });
    save(&r, "openai", "creative", &preset).unwrap();
    let back = load(&r, "openai", "creative").unwrap();
    assert_eq!(back, preset);
    // 文件落到了 OpenAI Settings 子目录
    assert!(r.join("OpenAI Settings").join("creative.json").exists());
}

#[test]
fn list_returns_names_without_extension() {
    let r = root("list");
    save(&r, "instruct", "alice", &json!({"a":1})).unwrap();
    save(&r, "instruct", "bob", &json!({"b":2})).unwrap();
    // 其他 apiId 不应混入
    save(&r, "openai", "carol", &json!({"c":3})).unwrap();
    // 无关文件应被忽略
    std::fs::write(r.join("instruct").join("README.md"), b"hi").unwrap();
    let mut got = list(&r, "instruct").unwrap();
    got.sort();
    assert_eq!(got, vec!["alice".to_string(), "bob".to_string()]);
}

#[test]
fn list_empty_when_no_directory() {
    let r = root("empty");
    let got = list(&r, "reasoning").unwrap();
    assert!(got.is_empty());
}

#[test]
fn delete_existing_returns_ok_missing_returns_not_found() {
    let r = root("delete");
    save(&r, "context", "trim", &json!({"x":1})).unwrap();
    delete(&r, "context", "trim").unwrap();
    assert!(!r.join("context").join("trim.json").exists());
    let err = delete(&r, "context", "trim").unwrap_err();
    assert!(matches!(err, tavern_presets::PresetError::NotFound(_)));
}

#[test]
fn unknown_api_id_errors() {
    let r = root("unknown");
    let err = save(&r, "bogus", "x", &json!({})).unwrap_err();
    assert!(matches!(err, tavern_presets::PresetError::UnknownApiId(_)));
    let err = load(&r, "bogus", "x").unwrap_err();
    assert!(matches!(err, tavern_presets::PresetError::UnknownApiId(_)));
    let err = list(&r, "bogus").unwrap_err();
    assert!(matches!(err, tavern_presets::PresetError::UnknownApiId(_)));
}

#[test]
fn path_traversal_is_rejected() {
    let r = root("traversal");
    let err = save(&r, "openai", "../escape", &json!({})).unwrap_err();
    assert!(matches!(err, tavern_presets::PresetError::Storage(_)));
    let err = save(&r, "openai", "a/b", &json!({})).unwrap_err();
    assert!(matches!(err, tavern_presets::PresetError::Storage(_)));
    // 文件确实没被写出到任何位置
    assert!(!r.join("escape.json").exists());
    assert!(!r.join("a").exists());
}

#[test]
fn restore_returns_empty_default_without_builtin() {
    let r = root("restore");
    let out = restore(&r, "openai", "anything");
    assert!(!out.is_default);
    assert_eq!(out.preset, json!({}));
}

#[test]
fn save_is_atomic_replacing_existing_content() {
    let r = root("atomic");
    save(&r, "sysprompt", "sp", &json!({"v": 1})).unwrap();
    save(&r, "sysprompt", "sp", &json!({"v": 2})).unwrap();
    assert_eq!(load(&r, "sysprompt", "sp").unwrap(), json!({"v": 2}));
}

#[test]
fn preset_dirs_match_sillytavern_layout() {
    // ensure() 建出的目录必须与读写路径一致，否则 ST 数据导入后 list 为空。
    let r = root("layout");
    let dirs = tavern_storage::DataRoot::new(&r).user("default-user");
    dirs.ensure().unwrap();
    for api in ["openai", "kobold", "novel", "textgenerationwebui"] {
        let folder = tavern_presets::folder_for(api).unwrap();
        assert!(
            dirs.root().join(folder).is_dir(),
            "ensure() must create `{folder}` used by folder_for(`{api}`)"
        );
    }
}

#[test]
fn list_skips_hidden_and_dangling_entries() {
    let r = root("hidden");
    save(&r, "instruct", "real", &json!({"a": 1})).unwrap();
    let dir = r.join("instruct");
    std::fs::write(dir.join(".hidden.json"), b"{}").unwrap();
    std::os::unix::fs::symlink(dir.join("missing-target.json"), dir.join("dangling.json")).unwrap();

    let got = list(&r, "instruct").unwrap();
    assert_eq!(got, vec!["real".to_string()]);
}
