//! 兑换码 wire 层单测:`RedemptionView` 的 serde 形状 + 页面映射纯函数。
//!
//! 纯函数/DTO 断言,不依赖网络与 Dioxus runtime。

use admin_page_admin::api::RedemptionView;
use admin_page_admin::shared::{RedRowFE, map_redemption_view};

fn view() -> RedemptionView {
    RedemptionView {
        key: "key-1".into(),
        code_preview: "fx-086c****".into(),
        quota: 1_000_000,
        status: 1,
        redeemed_by: None,
        redeemed_at: None,
        created_at: "2026-09-01 00:00:00".into(),
    }
}

/// 后端列表端点 `{"items":[...]}` 里单条兑换码的 camelCase 字段形状。
#[test]
fn redemption_view_deserializes_camel_case() {
    let raw = r#"{
        "key": "key-1",
        "codePreview": "fx-086c****",
        "quota": 1000000,
        "status": 1,
        "redeemedBy": "admin",
        "redeemedAt": "2026-09-02 10:00:00",
        "createdAt": "2026-09-01 00:00:00"
    }"#;
    let v: RedemptionView = serde_json::from_str(raw).expect("camelCase 字段应可解码");
    assert_eq!(v.key, "key-1");
    assert_eq!(v.code_preview, "fx-086c****");
    assert_eq!(v.quota, 1_000_000);
    assert_eq!(v.status, 1);
    assert_eq!(v.redeemed_by.as_deref(), Some("admin"));
    assert_eq!(v.redeemed_at.as_deref(), Some("2026-09-02 10:00:00"));
    assert_eq!(v.created_at, "2026-09-01 00:00:00");
}

/// camelCase 命名:snake_case 的 `code_preview` 直接投喂应缺字段失败。
#[test]
fn redemption_view_rejects_snake_case() {
    let raw = r#"{
        "key": "key-1",
        "code_preview": "fx-086c****",
        "quota": 1,
        "status": 1,
        "createdAt": "2026-09-01 00:00:00"
    }"#;
    let v = serde_json::from_str::<RedemptionView>(raw);
    assert!(v.is_err(), "后端是 camelCase,wire 层不接受 snake_case");
}

/// 字段缺省:核销信息缺省为 None,列表端点的最小载荷应可解码。
#[test]
fn redemption_view_tolerates_minimal_payload() {
    let raw = r#"{
        "key": "key-1",
        "codePreview": "fx-086c****",
        "quota": 500000,
        "status": 2,
        "createdAt": "2026-09-01 00:00:00"
    }"#;
    let v: RedemptionView = serde_json::from_str(raw).expect("核销信息缺省应合法");
    assert_eq!(v.redeemed_by, None);
    assert_eq!(v.redeemed_at, None);
}

/// quota 换算:后端内部计费单位 500000 = ¥1。
#[test]
fn map_converts_quota_units_to_cny() {
    let row = map_redemption_view(view());
    assert!(
        (row.quota_cny - 2.0).abs() < 1e-9,
        "1_000_000 内部单位 = ¥2,实际 {quota}",
        quota = row.quota_cny
    );
}

/// 字段直通:key / 预览 / 状态 / 创建时间原样映射。
#[test]
fn map_passthrough_fields() {
    let mut v = view();
    v.status = 3;
    v.redeemed_by = Some("alice".into());
    v.redeemed_at = Some("2026-09-02 10:00:00".into());
    let row = map_redemption_view(v);
    assert_eq!(row.key, "key-1");
    assert_eq!(row.code_preview, "fx-086c****");
    assert_eq!(row.status, 3);
    assert_eq!(row.redeemed_by.as_deref(), Some("alice"));
    assert_eq!(row.redeemed_at, "2026-09-02 10:00:00");
    assert_eq!(row.created, "2026-09-01 00:00:00");
}

/// 未核销的码:redeemed_at 缺省映射为空串(卡片据此隐藏核销时间行)。
#[test]
fn map_default_empty_redeemed_at() {
    let row = map_redemption_view(view());
    assert_eq!(row.redeemed_at, "");
    assert_eq!(row.redeemed_by, None);
}

/// RedRowFE 是展示模型:PartialEq 供组件内断言与 diff 使用。
#[test]
fn row_fe_partial_eq() {
    let a = map_redemption_view(view());
    let b = map_redemption_view(view());
    assert_eq!(a, b);
    let mut other: RedRowFE = a;
    other.status = 2;
    assert_ne!(other, b);
}
