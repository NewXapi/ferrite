//! 设置读写与未知字段保留。

#[test]
fn missing_file_reads_as_empty_object() {
    let p = std::env::temp_dir().join("ferrite-nope-settings.json");
    std::fs::remove_file(&p).ok();
    assert_eq!(tavern_settings::load(&p).unwrap(), serde_json::json!({}));
}

#[test]
fn unknown_fields_are_preserved() {
    let p = std::env::temp_dir().join(format!("ferrite-set-{}-preserve.json", std::process::id()));
    let v = serde_json::json!({"temperature": 0.7, "future_field": {"a": 1}});
    tavern_settings::save(&p, &v).unwrap();
    assert_eq!(tavern_settings::load(&p).unwrap(), v);
    std::fs::remove_file(&p).ok();
}
