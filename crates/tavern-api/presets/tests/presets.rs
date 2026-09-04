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

// ---------------------------------------------------------------------------
// HTTP 线格式契约
//
// 前端发的是 `apiId`（camelCase），Rust 侧字段是 `api_id`。这层靠 serde
// `rename` 搭桥：写错一个字母，请求就直接 422，而上面那些库层测试一个都
// 抓不到。所以这里锁死 DTO 的反序列化形状。
// ---------------------------------------------------------------------------

#[test]
fn list_query_accepts_camel_case_api_id() {
    let q: tavern_presets::http::ListQuery =
        serde_json::from_value(json!({ "apiId": "openai" })).expect("camelCase apiId must parse");
    assert_eq!(q.api_id, "openai");
}

#[test]
fn list_query_rejects_snake_case_api_id() {
    // 若哪天 rename 被删掉，这条会失败，提醒线格式变了。
    let parsed =
        serde_json::from_value::<tavern_presets::http::ListQuery>(json!({ "api_id": "openai" }));
    assert!(parsed.is_err(), "wire contract is apiId, not api_id");
}

#[test]
fn save_body_carries_api_id_name_and_preset() {
    let body: tavern_presets::http::SaveBody = serde_json::from_value(json!({
        "apiId": "instruct",
        "name": "alice",
        "preset": { "temperature": 0.7 }
    }))
    .expect("save body must parse");
    assert_eq!(body.api_id, "instruct");
    assert_eq!(body.name, "alice");
    assert_eq!(body.preset["temperature"], json!(0.7));
}

#[test]
fn delete_query_needs_both_api_id_and_name() {
    let q: tavern_presets::http::DeleteQuery =
        serde_json::from_value(json!({ "apiId": "context", "name": "trim" }))
            .expect("delete query must parse");
    assert_eq!((q.api_id.as_str(), q.name.as_str()), ("context", "trim"));

    // 缺 name 必须拒绝，否则会删掉意料之外的东西。
    assert!(
        serde_json::from_value::<tavern_presets::http::DeleteQuery>(json!({ "apiId": "context" }))
            .is_err()
    );
}

#[test]
fn restore_body_parses_and_response_uses_camel_case() {
    let body: tavern_presets::http::RestoreBody =
        serde_json::from_value(json!({ "apiId": "openai", "name": "default" }))
            .expect("restore body must parse");
    assert_eq!(body.api_id, "openai");

    // 响应侧同样是 camelCase：前端读 `isDefault`。
    let wire = serde_json::to_value(restore(&root("wire"), &body.api_id, &body.name)).unwrap();
    assert_eq!(wire["isDefault"], json!(false));
    assert_eq!(wire["preset"], json!({}));
    assert!(wire.get("is_default").is_none());
}
