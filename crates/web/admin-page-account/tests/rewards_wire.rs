//! RewardsPanel 所接 billing 端点的 wire 契约测试:前端 DTO 与后端 handler
//! 实际响应形状逐字对账 (`admin-billing`: wallet.rs `{"wallet":…}` /
//! affiliate.rs `{"overview":…}` / redeem.rs `{"key"}→{"quota","success"}` /
//! topup.rs `{"order_id":…}`)。后端字段改名或前端误写 snake_case 时红。
//!
//! 兑换请求体与 quota 提取的既有对账在 tests/wire_shapes.rs
//! (UserTopupRequest + topup_credited_quota),此处不重复。

use admin_page_account::api;
use client::{
    AffiliateOverviewResponse, OpenTopupRequest, RedeemRequest, TopupOrder, WalletResponse,
};

#[test]
fn wallet_response_decodes_multi_currency_camel_case() {
    // 后端 contract::WalletView 是 #[serde(rename_all="camelCase")]:
    // userKey / balances[].currencyCode / amount / availableI64 必须逐字命中。
    // 多币种行按序保留 (面板逐行渲染,顺序即后端返回顺序)。
    let json = r#"{
        "wallet": {
            "userKey": "8b1c-uuid",
            "balances": [
                { "currencyCode": "FREE", "amount": 123 },
                { "currencyCode": "GOLD", "amount": 7 }
            ],
            "availableI64": 130
        }
    }"#;
    let resp: WalletResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.wallet.user_key, "8b1c-uuid");
    assert_eq!(resp.wallet.available_i64, 130);
    let codes: Vec<_> = resp
        .wallet
        .balances
        .iter()
        .map(|b| (b.currency_code.as_str(), b.amount))
        .collect();
    assert_eq!(codes, [("FREE", 123i64), ("GOLD", 7i64)]);
}

#[test]
fn wallet_empty_balances_is_normal_state() {
    // 新账号未 seed / 余额耗尽:balances=[] 是合法空态 (面板渲染虚线占位),
    // 不是解码错误,availableI64=0 如实展示。
    let resp: WalletResponse =
        serde_json::from_str(r#"{"wallet":{"userKey":"u","balances":[],"availableI64":0}}"#)
            .unwrap();
    assert!(resp.wallet.balances.is_empty());
    assert_eq!(resp.wallet.available_i64, 0);
}

#[test]
fn wallet_missing_fields_and_unknown_extras_tolerated() {
    // struct 级 #[serde(default)]:缺字段 / 空对象 (信封 data:null 兜底路径)
    // 解码成全零视图而非报错,面板显示空态,不断言炸整页。
    let empty: WalletResponse = serde_json::from_str("{}").unwrap();
    assert_eq!(empty.wallet.user_key, "");
    assert!(empty.wallet.balances.is_empty());
    assert_eq!(empty.wallet.available_i64, 0);
    // 后端未来新增字段 (外层或余额行内) 必须被忽略,不打破现有解码。
    let extra: WalletResponse = serde_json::from_str(
        r#"{"wallet":{"availableI64":5,"newFlag":true},"future":{"x":1},
             "balances_dummy":0}"#,
    )
    .unwrap();
    assert_eq!(extra.wallet.available_i64, 5);
}

#[test]
fn affiliate_overview_response_decodes_wrapper_key() {
    // 后端 overview handler 返回 {"overview": {userKey, inviteCount, totalReward}}:
    // 包装 key "overview" 与 camelCase 字段名是面板取数的全部依赖。
    let json = r#"{"overview":{"userKey":"u-1","inviteCount":3,"totalReward":9000}}"#;
    let resp: AffiliateOverviewResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.overview.user_key, "u-1");
    assert_eq!(resp.overview.invite_count, 3);
    assert_eq!(resp.overview.total_reward, 9000);
}

#[test]
fn affiliate_overview_missing_fields_default_to_zero() {
    // 后端统计侧是占位实现 (affiliate.rs TODO(affiliate-stats)),缺省字段
    // 走 default → 0/空串:面板显示 0 (真实值),不 panic、不造数。
    let resp: AffiliateOverviewResponse = serde_json::from_str(r#"{"overview":{}}"#).unwrap();
    assert_eq!(resp.overview.user_key, "");
    assert_eq!(resp.overview.invite_count, 0);
    assert_eq!(resp.overview.total_reward, 0);
}

#[test]
fn open_topup_request_serializes_exact_camel_case() {
    // 后端 topup.rs 用 contract TopUpRequest (camelCase rename) 反序列化:
    // wire 必须是 userKey/currency/amount(+provider) 字段,误写 snake_case 即 422。
    // provider: None 时不发送该键 (skip_serializing_if) —— manual 与后端缺省同义。
    let req = OpenTopupRequest {
        user_key: "u-9".into(),
        currency: "FREE".into(),
        amount: 100,
        provider: None,
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(
        v,
        serde_json::json!({ "userKey": "u-9", "currency": "FREE", "amount": 100 })
    );
    // 指定真渠道时该键出现,epay 路径靠它分流。
    let req = OpenTopupRequest {
        user_key: "u-9".into(),
        currency: "FREE".into(),
        amount: 10,
        provider: Some("epay".into()),
    };
    assert_eq!(
        serde_json::to_value(&req).unwrap(),
        serde_json::json!({ "userKey": "u-9", "currency": "FREE", "amount": 10, "provider": "epay" })
    );
}

#[test]
fn topup_order_response_keeps_snake_case_order_id() {
    // 后端 handler 序列化 OpenTopupResult — 键逐字 snake_case,
    // DTO 若误加 camelCase rename 这条第一时间红。
    let ok: TopupOrder = serde_json::from_str(r#"{"order_id":"abc-123"}"#).unwrap();
    assert_eq!(ok.order_id.as_deref(), Some("abc-123"));
    // 缺 order_id → None:面板降级为无单号文案,不假造单号。
    let missing: TopupOrder = serde_json::from_str("{}").unwrap();
    assert_eq!(missing.order_id, None);
    // 真渠道开单带 payment_url → 「去支付」按钮渲染;manual 响应省略该键 → None。
    let paid: TopupOrder = serde_json::from_str(
        r#"{"order_id":"abc-123","payment_url":"https://pay.example.com/mapi.php?m=buy"}"#,
    )
    .unwrap();
    assert_eq!(paid.order_id.as_deref(), Some("abc-123"));
    assert_eq!(
        paid.payment_url.as_deref(),
        Some("https://pay.example.com/mapi.php?m=buy")
    );
    assert_eq!(missing.payment_url, None);
}

#[test]
fn redeem_request_body_is_single_key_field() {
    // redeem.rs 本地 TopupRequest { key } 无 rename:请求体有且只有 "key"。
    // (与 wire_shapes 的 contract UserTopupRequest 对账同源,双保险:
    // 面板实际发送的是本 DTO,不是 contract 类型。)
    let v = serde_json::to_value(RedeemRequest {
        key: "fx-abc".into(),
    })
    .unwrap();
    let obj = v.as_object().expect("redeem 请求体必须是 JSON object");
    assert_eq!(obj.len(), 1, "redeem 请求体只含 key 一个字段");
    assert_eq!(obj["key"], serde_json::json!("fx-abc"));
}

#[test]
fn redeem_success_shape_feeds_quota_extractor() {
    // 面板兑换成功路径: {"quota": <i64>, "success": true} → 提取真实入账值;
    // 非整数 quota 必须得 None (降级通用文案, 不假造数值)。
    let v: serde_json::Value =
        serde_json::from_str(r#"{"quota": 500000, "success": true}"#).unwrap();
    assert_eq!(api::topup_credited_quota(&v), Some(500_000));
    let dirty: serde_json::Value =
        serde_json::from_str(r#"{"quota": "500000", "success": true}"#).unwrap();
    assert_eq!(api::topup_credited_quota(&dirty), None);
}
