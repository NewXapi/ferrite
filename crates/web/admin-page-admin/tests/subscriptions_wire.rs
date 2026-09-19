//! 订阅页 wire 层单测：`SubscriptionView` 的 serde 形状 +
//! `SubscriptionView → PlanRow` 映射 + `SubscriptionUpsertRequest` 请求体形状。
//!
//! 纯函数/DTO 断言，不依赖网络与 Dioxus runtime。两端形状漂移（后端
//! `admin_billing::subscriptions::SubscriptionView` 的 flatten 输出 vs 前端
//! `admin_page_admin::api::SubscriptionView` 的解码）由本文件钉死——前端不能
//! `use` 后端服务层类型（admin-page-admin 不依赖 admin-billing），只能靠
//! 形状断言保证 JSON 契约一致。

use admin_page_admin::api::SubscriptionView;
use admin_page_admin::state::{PlanRow, map_subscription_view};
use contract::api::billing::SubscriptionUpsertRequest;

/// 后端真实输出形状：`SubscriptionDto` 之上 flatten key/currency/createdAt/
/// updatedAt（camelCase）。后端只填表里有列的字段（name/price/quota/group/
/// periodVal/periodUnit/maxPerUser/enabled/sortOrder），其余 DTO 字段恒为 null。
fn sample_view_json() -> serde_json::Value {
    serde_json::json!({
        "key": "0b3f2c1a-0000-4000-8000-000000000001",
        "id": null,
        "name": "月度会员",
        "description": null,
        "price": 9.9,
        "quota": 50.0,
        "currencyPrice": null,
        "paymentMethod": null,
        "group": "vip",
        "downgradeGroup": null,
        "periodVal": 30,
        "periodUnit": "days",
        "resetCycle": null,
        "priority": null,
        "enabled": true,
        "allowRedeem": null,
        "allowWallet": null,
        "maxPerUser": 1,
        "sortOrder": 2,
        "stripePriceId": null,
        "creemProductId": null,
        "waffoProductId": null,
        "currency": "CNY",
        "createdAt": "2026-09-17T00:00:00Z",
        "updatedAt": "2026-09-17T12:00:00Z"
    })
}

/// camelCase 全字段载荷可解码，且后端「有列」与「无列」的字段口径分开断言。
#[test]
fn subscription_view_deserializes_camel_case() {
    let v: SubscriptionView = serde_json::from_value(sample_view_json()).unwrap();
    assert_eq!(v.key, "0b3f2c1a-0000-4000-8000-000000000001");
    assert_eq!(v.name, "月度会员");
    assert_eq!(v.price, Some(9.9));
    assert_eq!(v.quota, Some(50.0));
    assert_eq!(v.group.as_deref(), Some("vip"));
    assert_eq!(v.period_val, Some(30));
    assert_eq!(v.period_unit.as_deref(), Some("days"));
    assert_eq!(v.max_per_user, Some(1));
    assert_eq!(v.sort_order, Some(2));
    assert_eq!(v.enabled, Some(true));
    assert_eq!(v.currency, "CNY");
    assert_eq!(v.created_at, "2026-09-17T00:00:00Z");
    assert_eq!(v.updated_at, "2026-09-17T12:00:00Z");
    // ferrite 侧无对应表列的 DTO 字段恒为 None。
    assert_eq!(v.id, None);
    assert_eq!(v.description, None);
    assert_eq!(v.payment_method, None);
    assert_eq!(v.stripe_price_id, None);
}

/// snake_case 的额外字段被 serde 忽略（不报错、也不填进 camelCase 槽位）；
/// 缺必填的 name 则必须解码失败——client 层据此返回 ApiError::Decode，
/// 页面走错误态而不是静默渲染空行。
#[test]
fn subscription_view_ignores_snake_case_and_requires_name() {
    let v: SubscriptionView = serde_json::from_value(serde_json::json!({
        "key": "k1",
        "name": "月度会员",
        "created_at": "2026-09-17T00:00:00Z"
    }))
    .expect("未知 snake_case 字段应被忽略");
    assert_eq!(
        v.created_at, "",
        "snake_case 键不得填进 camelCase 槽位（created_at ≠ createdAt）"
    );

    let missing_name = serde_json::from_value::<SubscriptionView>(serde_json::json!({"key": "k1"}));
    assert!(missing_name.is_err(), "缺 name 必须解码失败");
}

/// 最小载荷容错：DTO 字段缺席 → None；外层 flatten 字段缺省 → 默认空串。
/// 列表端点实际总带这四项，但前端解码不应因后端少字段而整体崩掉。
#[test]
fn subscription_view_tolerates_minimal_payload() {
    let v: SubscriptionView =
        serde_json::from_value(serde_json::json!({"key": "k1", "name": "月度会员"}))
            .expect("key + name 应足够解码");
    assert_eq!(v.key, "k1");
    assert_eq!(v.name, "月度会员");
    assert_eq!(v.quota, None);
    assert_eq!(v.enabled, None);
    assert_eq!(v.currency, "");
}

/// 列表端点 `{"items":[..],"total":n}` 壳的解码形状（对齐
/// `list_subscriptions_api` 的剥壳方式：取 items、忽略 total）。
#[derive(Debug, Default, serde::Deserialize)]
struct SubscriptionItemsMirror {
    #[serde(default)]
    items: Vec<SubscriptionView>,
    #[serde(default)]
    total: u64,
}

#[test]
fn list_envelope_decodes_items() {
    let raw = serde_json::json!({ "items": [sample_view_json()], "total": 1 });
    let r: SubscriptionItemsMirror = serde_json::from_value(raw).unwrap();
    assert_eq!(r.total, 1);
    assert_eq!(r.items.len(), 1);
    assert_eq!(r.items[0].name, "月度会员");
    assert_eq!(r.items[0].key, "0b3f2c1a-0000-4000-8000-000000000001");
}

/// POST 端点 `{"subscription": <view>}` 壳的解码形状（对齐
/// `upsert_subscription_api` 的剥壳方式）。
#[derive(Debug, Default, serde::Deserialize)]
struct SubscriptionRespMirror {
    #[serde(default)]
    subscription: SubscriptionView,
}

#[test]
fn post_envelope_decodes_subscription() {
    let raw = serde_json::json!({ "subscription": sample_view_json() });
    let r: SubscriptionRespMirror = serde_json::from_value(raw).unwrap();
    assert_eq!(r.subscription.key, "0b3f2c1a-0000-4000-8000-000000000001");
    assert_eq!(r.subscription.name, "月度会员");
}

/// 映射字段对齐：key 透传（DELETE 定位符）、title = name、price 取 f64、
/// period_val = 有效期天数、group = 升级分组、max_per_user = 限购、
/// enabled 直映、id 透传 None。
#[test]
fn map_subscription_view_aligns_fields() {
    let v: SubscriptionView = serde_json::from_value(sample_view_json()).unwrap();
    let row: PlanRow = map_subscription_view(v);
    assert_eq!(row.key, "0b3f2c1a-0000-4000-8000-000000000001");
    assert_eq!(row.title, "月度会员");
    assert!((row.price - 9.9).abs() < 1e-9);
    assert_eq!(row.period_val, 30);
    assert_eq!(row.period_unit, "days");
    assert_eq!(row.group, "vip");
    assert_eq!(row.max_per_user, 1);
    assert_eq!(row.sort_order, 2);
    assert!(row.enabled);
    assert_eq!(row.currency, "CNY");
    assert_eq!(row.id, None);
    // 后端不填的展示字段回落默认值，绝不用本地假数据回填（订阅页非演示态）。
    assert_eq!(row.subtitle, "");
    assert_eq!(row.payment_method, "");
    assert_eq!(row.stripe_price_id, "");
}

/// quota 展示口径不换算：后端入库侧 ×500_000、读回侧 ÷500_000 已对称还原，
/// 前端拿到的 50.0 就是展示值——映射再乘一次会得到 25_000_000 的错误额度。
#[test]
fn map_preserves_quota_display_scale() {
    let v: SubscriptionView = serde_json::from_value(sample_view_json()).unwrap();
    let row = map_subscription_view(v);
    assert!(
        (row.quota - 50.0).abs() < 1e-9,
        "quota 必须保持展示口径 50.0，实际 {q}",
        q = row.quota
    );
}

/// None / 空字段回落：空升级分组映射为空串（页面据此隐藏分组徽标），
/// currency 缺省回退 CNY（价格符号兜底，避免空符号渲染）。
#[test]
fn map_defaults_none_fields() {
    let v: SubscriptionView = serde_json::from_value(serde_json::json!({
        "key": "k2",
        "name": "免费套餐",
        "quota": 0.0,
        "enabled": false
    }))
    .unwrap();
    let row = map_subscription_view(v);
    assert_eq!(row.group, "");
    assert_eq!(row.currency, "CNY");
    assert_eq!(row.period_val, 0);
    assert!(!row.enabled);
    assert_eq!(row.max_per_user, 0);
}

/// upsert 请求体形状：camelCase，price 是**字符串**（NUMERIC 语义，避免
/// 浮点误差，后端 parse 失败即 400），quota 是数字。
#[test]
fn upsert_request_serializes_camel_case_with_string_price() {
    let req = SubscriptionUpsertRequest {
        name: "月度会员".into(),
        price: "9.9".into(),
        currency: "CNY".into(),
        duration_days: 30,
        quota: 50.0,
        upgrade_group: Some("vip".into()),
        max_purchases: Some(1),
        enabled: Some(true),
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["name"], "月度会员");
    assert_eq!(v["price"], "9.9", "price 必须序列化为字符串");
    assert_eq!(v["currency"], "CNY");
    assert_eq!(v["durationDays"], 30);
    assert_eq!(v["quota"], 50.0);
    assert_eq!(v["upgradeGroup"], "vip");
    assert_eq!(v["maxPurchases"], 1);
    assert_eq!(v["enabled"], true);
    // snake_case 键不得出现：后端按 camelCase 反序列化，蛇形键会被当成
    // 未知字段丢弃，对应列静默保持旧值。
    assert!(v.get("duration_days").is_none());
    assert!(v.get("upgrade_group").is_none());
    assert!(v.get("max_purchases").is_none());
}

/// 可选字段缺席语义：不升级分组 / 不限购 / 启用缺省用 null 表达——
/// 后端 upgrade_group/max_purchases 列可空，enabled 缺省回 true。
#[test]
fn upsert_request_optional_fields_serialize_as_null() {
    let req = SubscriptionUpsertRequest {
        name: "重置分组".into(),
        price: "0".into(),
        currency: "CNY".into(),
        duration_days: 1,
        quota: 0.0,
        upgrade_group: None,
        max_purchases: None,
        enabled: None,
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["upgradeGroup"], serde_json::Value::Null);
    assert_eq!(v["maxPurchases"], serde_json::Value::Null);
    assert_eq!(v["enabled"], serde_json::Value::Null);
}
