//! 兑换码 wire 层单测:`RedemptionView` 的 serde 形状 + 页面映射纯函数。
//!
//! 纯函数/DTO 断言,不依赖网络与 Dioxus runtime。

use admin_page_admin::api::RedemptionView;
use admin_page_admin::redemptions::{RedRowFE, map_redemption_view};

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

/// 状态展示语义回归闸:与后端 redeem.rs 写侧一致——核销 SET 2、停用 SET 3。
/// 历史事故:本页曾把 2/3 的文案/统计对调(管理员看到核销↔停用整体颠倒)。
/// 语义由 tests/manage_wire_contract.rs 在后端钉死,此处钉前端,防再翻。
#[test]
fn status_display_matches_backend_semantics() {
    use admin_page_admin::redemptions::status_display;

    let unused = status_display(1);
    assert_eq!(unused.label, "未使用");
    assert_eq!(unused.bar_pct, 100);

    // 2 = 已核销(CAS 核销终态):面额耗尽,不是停用。
    let redeemed = status_display(2);
    assert_eq!(redeemed.label, "已核销");
    assert_eq!(redeemed.bar_pct, 0);

    // 3 = 已停用(DELETE 终态):面额冻结,不是核销。
    let disabled = status_display(3);
    assert_eq!(disabled.label, "已停用");
    assert_eq!(disabled.bar_pct, 40);

    // 未知值兜底落「已停用」而非「已核销」——保守展示不夸大剩余面额。
    assert_eq!(status_display(9).label, "已停用");
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
