//! 渠道编辑 PUT（`UpdateChannelBody`）的最小 diff 体形状断言。
//!
//! 后端 `ChannelService::update`（admin-catalog channels.rs）的列语义：
//! - `keys: Option<Vec<String>>` 缺席 = 保持现有密钥；恒发 `[]` 会被
//!   validate 以 "at least one key required" 拒绝 → 历史上编辑弹窗不重输
//!   密钥必然 400。
//! - `test_model` 列 SQL 直绑、**无** COALESCE：缺席即把列清成 NULL，
//!   所以请求体必须恒带现值（含现值为 null 的情况）。
//! - `models`/`priority`/`weight`/`status` 走 COALESCE：弹窗不管理，
//!   一律不发，历史全量体（恒带 `models:[]`、`priority:0`、`weight:0`）
//!   会把这些列静默清零。
//!
//! 本文件用序列化结果钉死以上四条，防止调用点或结构体回退到全量裸体。

use admin_page_admin::api::UpdateChannelBody;
use serde_json::{Value, json};

/// 弹窗编辑态的基准 body：字段值即 channels.rs `do_submit` 从表单信号取到的形状。
fn base_body(keys: Option<Vec<String>>) -> UpdateChannelBody {
    UpdateChannelBody {
        name: "OpenAI 官方".into(),
        channel_type: "openai".into(),
        base_url: "https://api.openai.com/v1".into(),
        groups: vec!["default".into()],
        remark: String::new(),
        test_model: None,
        keys,
    }
}

/// 序列化并转成对象视图（wire 语义：`None`/缺席字段应整体不出现在 map 中）。
fn to_map(body: &UpdateChannelBody) -> serde_json::Map<String, Value> {
    let v = serde_json::to_value(body).expect("UpdateChannelBody 必须可序列化");
    match v {
        Value::Object(m) => m,
        other => panic!("请求体应为 JSON object，实际为 {other:?}"),
    }
}

/// 用户未重输密钥（textarea 解析结果为空 Vec）→ `keys` 字段整体缺席，
/// 后端 `None` 分支保持现有密钥。复刻调用点的 `(!k.is_empty()).then_some(k)`。
#[test]
fn omitted_keys_field_absent_so_backend_keeps_existing() {
    let parsed: Vec<String> = Vec::new(); // 空文本框按行解析的产物
    let body = base_body((!parsed.is_empty()).then_some(parsed));
    let m = to_map(&body);
    assert!(
        !m.contains_key("keys"),
        "未重输密钥时 keys 字段必须整体缺席（发 [] 会被后端 400 拒绝）: {m:?}"
    );
}

/// 用户重输明文密钥 → keys 原样携带（后端整体替换语义）。
#[test]
fn reentered_keys_carried_verbatim() {
    let body = base_body(Some(vec!["sk-plain-a".into(), "sk-plain-b".into()]));
    let m = to_map(&body);
    assert_eq!(
        m.get("keys"),
        Some(&json!(["sk-plain-a", "sk-plain-b"])),
        "重输的明文密钥应逐条原样携带"
    );
}

/// `testModel` 恒发：现值为 None 时也必须序列化成显式 `null`——
/// 该列无 COALESCE，字段缺席与 null 同为清 NULL，恒带现值才能保证
/// 「现值非 NULL 的渠道保存后不被误清」，且 null 情况形状上有契约锚点。
#[test]
fn test_model_always_present_including_null() {
    let m = to_map(&base_body(None));
    assert!(
        m.contains_key("testModel"),
        "testModel 必须恒发（直绑列，缺席即清）: {m:?}"
    );
    assert_eq!(
        m.get("testModel"),
        Some(&Value::Null),
        "现值 None → 显式 null"
    );

    let mut with_tm = base_body(None);
    with_tm.test_model = Some("gpt-4o".into());
    let m = to_map(&with_tm);
    assert_eq!(
        m.get("testModel"),
        Some(&json!("gpt-4o")),
        "现值 Some 必须原样回传"
    );
}

/// 弹窗不管理的列（models/priority/weight/status）绝不出现在请求体里，
/// 交给后端 COALESCE 保持现值；弹窗管理的列必须出现。
#[test]
fn unmanaged_columns_absent_edited_columns_present() {
    let m = to_map(&base_body(None));
    for absent in ["models", "priority", "weight", "status"] {
        assert!(
            !m.contains_key(absent),
            "弹窗不管理的列 {absent} 不应随最小 diff 体发出（全量裸体会静默清零该列）"
        );
    }
    for present in ["name", "channelType", "baseUrl", "groups", "remark"] {
        assert!(m.contains_key(present), "弹窗编辑项 {present} 必须恒发");
    }
}

/// 掩码值绝不回传：列表接口下发的 ChannelDto.keys 是掩码
/// （形如 `sk-ab****ef`），编辑弹窗密钥框留空 → body 不含 keys 字段，
/// 序列化结果里不得出现掩码串。
#[test]
fn masked_key_never_serialized() {
    // 模拟调用点现场：列表行里的掩码密钥 + 用户没有动文本框。
    let masked_in_row = vec!["sk-ab****ef".to_string()];
    let parsed_textarea: Vec<String> = Vec::new(); // 文本框为空
    let body = base_body((!parsed_textarea.is_empty()).then_some(parsed_textarea));
    let s = serde_json::to_string(&body).expect("序列化应成功");
    assert!(!s.contains("****"), "序列化结果不得含掩码串: {s}");
    for masked in &masked_in_row {
        assert!(
            !s.contains(masked),
            "列表行掩码 {masked} 不得出现在 PUT 请求体中: {s}"
        );
    }
}
