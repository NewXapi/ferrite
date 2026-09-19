//! 模型定价字段落库 — 逻辑层（无 PG）测试。
//!
//! 0018 给 api_models 加了 input_per_1k / output_per_1k / multiplier 三列；
//! 本文件只测**不落库**的三层形状与校验：
//! - `ModelView` 序列化形状（camelCase + 默认值 0/0/1.0）——前端别名页读这个；
//! - `validate_model` 对三字段的非负/有限/倍率上限校验；
//! - `UpdateModelRequest` 的 None 语义（缺字段 = 保持现值，不置零）以及
//!   与 contract `AliasUpsertRequest` 的线材兼容（前端 PUT body 能被后端
//!   反序列化成可写库的 Some 值——这正是此前「价格字段被读成 None 忽略」
//!   的回归保护）。
//!
//! 全部纯逻辑，无需 DATABASE_URL，故不加 `#[ignore]`。

use catalog::models::{CreateModelRequest, ModelView, UpdateModelRequest, validate_model};
use contract::api::billing::AliasUpsertRequest;
use serde_json::json;

/// 造一个定价全默认的 ModelView（模拟 0018 迁移后新插入的行）。
fn view_with_defaults() -> ModelView {
    ModelView {
        key: "11111111-2222-3333-4444-555555555555".into(),
        name: "gpt-4o".into(),
        owner: "owner".into(),
        model_type: "chat".into(),
        base_url: "https://api.example.com".into(),
        masked_key: "sk-1****5678".into(),
        capabilities: json!(["vision"]),
        speed: 100,
        rating: json!({}),
        usage_count: 0,
        max_tokens: 8192,
        is_vision: true,
        is_tool: false,
        status: 1,
        input_per_1k: 0.0,
        output_per_1k: 0.0,
        multiplier: 1.0,
        created_at: sqlx::types::chrono::Utc::now(),
        updated_at: sqlx::types::chrono::Utc::now(),
    }
}

/// 定价三元组的合法基线参数（其余字段全部合法），供校验测试复用。
const fn ok_pricing() -> (Option<f64>, Option<f64>, Option<f64>) {
    (Some(0.01), Some(0.02), Some(1.0))
}

// ---------- ModelView 序列化形状 ----------

#[test]
fn model_view_pricing_is_camel_case_with_defaults() {
    // 0018 默认值：未定价 0/0、无加价 1.0；前端按 camelCase 读这三个键。
    let v = serde_json::to_value(view_with_defaults()).expect("serialize");
    assert_eq!(v["inputPer1k"], json!(0.0));
    assert_eq!(v["outputPer1k"], json!(0.0));
    assert_eq!(v["multiplier"], json!(1.0));
    // snake_case 键不应出现——否则 rename_all = camelCase 失效，前端取不到值。
    assert!(v.get("input_per_1k").is_none());
    assert!(v.get("output_per_1k").is_none());
    // multiplier 两边拼写相同，只验证值正确即可。
    assert!(v.get("multiplier").is_some());
}

#[test]
fn model_view_custom_pricing_roundtrips() {
    let mut v = view_with_defaults();
    v.input_per_1k = 0.0175;
    v.output_per_1k = 0.07;
    v.multiplier = 1.25;
    let out = serde_json::to_value(&v).expect("serialize");
    assert_eq!(out["inputPer1k"], json!(0.0175));
    assert_eq!(out["outputPer1k"], json!(0.07));
    assert_eq!(out["multiplier"], json!(1.25));
}

// ---------- validate_model 定价校验 ----------

#[test]
fn validate_accepts_valid_and_absent_pricing() {
    let (i, o, m) = ok_pricing();
    validate_model("m", "o", "chat", "https://a.io", "sk-key", i, o, m).expect("valid pricing");
    // None = 未提供：create 走默认 / update 保持现值，校验必须放行。
    validate_model("m", "o", "chat", "https://a.io", "sk-key", None, None, None)
        .expect("absent pricing allowed");
    // 0 价格（未定价）与恰好 1.0 倍率是合法边界。
    validate_model(
        "m",
        "o",
        "chat",
        "https://a.io",
        "sk-key",
        Some(0.0),
        Some(0.0),
        Some(1.0),
    )
    .expect("zero prices allowed");
}

#[test]
fn validate_rejects_negative_pricing() {
    let (i, o, _) = ok_pricing();
    assert_bad_request(Some(-0.01), o, Some(1.0), "negative input");
    assert_bad_request(i, Some(-1.0), Some(1.0), "negative output");
    let (i, o, _) = ok_pricing();
    assert_bad_request(i, o, Some(-0.5), "negative multiplier");
}

#[test]
fn validate_rejects_nan_and_infinity() {
    // NaN/Infinity 若落库会被 JSON 序列化成非法字面量（serde_json 默认报错），
    // 且对运营无意义——在入口拒绝。其余字段给合法基线，确保拒绝确实来自坏值。
    assert_bad_request(Some(f64::NAN), Some(0.02), Some(1.0), "NaN input");
    assert_bad_request(
        Some(0.01),
        Some(f64::INFINITY),
        Some(1.0),
        "Infinity output",
    );
    assert_bad_request(
        Some(0.01),
        Some(0.02),
        Some(f64::NEG_INFINITY),
        "neg Infinity multiplier",
    );
}

#[test]
fn validate_enforces_multiplier_upper_bound() {
    // 上限边界值本身合法（含等号），防止运营手滑打出 1000x 天价倍率。
    let (i, o, _) = ok_pricing();
    validate_model(
        "m",
        "o",
        "chat",
        "https://a.io",
        "sk-key",
        i,
        o,
        Some(100.0),
    )
    .expect("multiplier at upper bound allowed");
    let (i, o, _) = ok_pricing();
    assert_bad_request(i, o, Some(100.1), "multiplier above upper bound");
}

/// 断言给定定价三元组被 `validate_model` 拒绝为 BadRequest（HTTP 400）。
fn assert_bad_request(input: Option<f64>, output: Option<f64>, mult: Option<f64>, what: &str) {
    let err = validate_model(
        "m",
        "o",
        "chat",
        "https://a.io",
        "sk-key",
        input,
        output,
        mult,
    )
    .expect_err(&format!("pricing should be rejected: {what}"));
    assert!(
        matches!(err, auth::AuthError::BadRequest(ref msg) if msg.contains("per_1k") || msg.contains("multiplier")),
        "expected BadRequest about pricing, got: {err:?}"
    );
}

// ---------- UpdateModelRequest None 语义 + 线材兼容 ----------

#[test]
fn update_request_absent_pricing_means_keep_current() {
    // 前端只改名字时 body 不含定价键 → None → COALESCE 保持现值，不置零。
    // 这是「None = 保持现值，不是置零」语义的形状证据。
    let req: UpdateModelRequest =
        serde_json::from_value(json!({"name": "renamed"})).expect("deserialize name-only update");
    assert_eq!(req.name.as_deref(), Some("renamed"));
    assert!(req.input_per_1k.is_none(), "absent input must stay None");
    assert!(req.output_per_1k.is_none(), "absent output must stay None");
    assert!(req.multiplier.is_none(), "absent multiplier must stay None");
}

#[test]
fn update_request_parses_pricing_from_camel_case() {
    let req: UpdateModelRequest = serde_json::from_value(json!({
        "inputPer1k": 0.035,
        "outputPer1k": 0.14,
        "multiplier": 1.5
    }))
    .expect("deserialize pricing update");
    assert_eq!(req.input_per_1k, Some(0.035));
    assert_eq!(req.output_per_1k, Some(0.14));
    assert_eq!(req.multiplier, Some(1.5));
}

#[test]
fn create_request_absent_pricing_is_none_and_optional_fields_default() {
    // 新建时缺席定价 = 未定价（默认 0/0/1.0 由 0018 DEFAULT 与 create 的
    // unwrap_or 兜底）；必填字段仍在（name/owner/api_key）。
    let req: CreateModelRequest = serde_json::from_value(json!({
        "name": "new-model", "owner": "o", "apiKey": "sk-xxxx"
    }))
    .expect("deserialize minimal create");
    assert_eq!(req.name, "new-model");
    assert_eq!(req.api_key, "sk-xxxx");
    assert!(req.input_per_1k.is_none());
    assert!(req.output_per_1k.is_none());
    assert!(req.multiplier.is_none());
}

/// 前端 `update_model_alias_api` 用 contract `AliasUpsertRequest` 序列化 PUT body；
/// 后端用 `UpdateModelRequest` 反序列化。此测试锁住两者的线材对齐：别名页发出的
/// 定价字段必须被后端读成 Some，而不是像此前一样因缺列/缺字段被吃成 None。
#[test]
fn alias_upsert_request_wire_is_compatible_with_update_request() {
    let alias = AliasUpsertRequest {
        name: "gpt-4o".into(),
        display_name: Some("GPT-4o".into()),
        input_per_1k: Some(0.0175),
        output_per_1k: Some(0.07),
        multiplier: Some(1.25),
        status: Some(1),
    };
    let body = serde_json::to_value(&alias).expect("serialize contract body");
    assert_eq!(
        body["inputPer1k"],
        json!(0.0175),
        "contract must emit camelCase"
    );

    let req: UpdateModelRequest =
        serde_json::from_value(body).expect("backend must parse contract body");
    assert_eq!(req.name.as_deref(), Some("gpt-4o"));
    assert_eq!(req.input_per_1k, Some(0.0175));
    assert_eq!(req.output_per_1k, Some(0.07));
    assert_eq!(req.multiplier, Some(1.25));
    assert_eq!(req.status, Some(1));
}
