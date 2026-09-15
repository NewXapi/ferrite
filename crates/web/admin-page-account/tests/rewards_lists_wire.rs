//! RewardsPanel 两个列表端点的 wire 契约测试: 前端 DTO 与后端 handler 实际响应
//! 形状逐字对账 (admin-billing: topup.rs `list_orders` / affiliate.rs invitees)。
//! 两个端点都是裸 `{"items":[...]}` 信封、camelCase 字段; 后端字段改名或前端
//! 误写 snake_case 时红。
//!
//! 面板取数路径: `client::fetch_topup_orders` / `client::fetch_invitees` 把响应
//! 反序列化成 `ListEnvelope<TopupOrderView|InviteeView>` 再拆 `items`,故此处
//! 直接对 `ListEnvelope` 解码 — 与运行时完全同构。
//!
//! 空数组是正常空态 (新账号无订单 / 无人受邀), 面板渲染虚线占位而非报错;
//! 单项契约 (兑换请求体 / quota 提取 / 开单 order_id) 见 tests/rewards_wire.rs。

use client::{InviteeView, ListEnvelope, TopupOrderView};

#[test]
fn topup_orders_decodes_camel_case_fields() {
    // 后端 contract::TopupOrderView 是 #[serde(rename_all="camelCase")]:
    // key / currency / amount / state / provider / createdAt 必须逐字命中;
    // createdAt 为 RFC3339 字符串, DTO 持 String (to_rfc3339() 原样保留)。
    let json = r#"{
        "items": [
            {
                "key": "ord-001",
                "currency": "FREE",
                "amount": 100,
                "state": "pending",
                "provider": "",
                "createdAt": "2026-09-14T10:20:30Z"
            }
        ]
    }"#;
    let resp: ListEnvelope<TopupOrderView> = serde_json::from_str(json).unwrap();
    assert_eq!(resp.items.len(), 1);
    let o = &resp.items[0];
    assert_eq!(o.key, "ord-001");
    assert_eq!(o.currency, "FREE");
    assert_eq!(o.amount, 100);
    assert_eq!(o.state, "pending");
    assert_eq!(o.provider, "");
    assert_eq!(o.created_at, "2026-09-14T10:20:30Z");
}

#[test]
fn topup_orders_state_string_preserved_verbatim() {
    // 状态机字符串原样保留 (pending|settling|paid|failed|refunded): 前端只展示
    // 不解释, 后端新增中间态不需要前端改解码就能渲染。
    for state in ["pending", "settling", "paid", "failed", "refunded"] {
        let json = format!(r#"{{"items":[{{"key":"k","state":"{state}"}}]}}"#);
        let resp: ListEnvelope<TopupOrderView> = serde_json::from_str(&json).unwrap();
        assert_eq!(resp.items[0].state, state);
    }
}

#[test]
fn topup_orders_empty_items_is_normal_state() {
    // 新账号无订单: {"items":[]} 是合法空态, 面板渲染「暂无充值记录」。
    let resp: ListEnvelope<TopupOrderView> = serde_json::from_str(r#"{"items":[]}"#).unwrap();
    assert!(resp.items.is_empty());
}

#[test]
fn topup_orders_missing_fields_default_and_extras_tolerated() {
    // struct 级 #[serde(default)]: 缺字段解码成空串/零而非报错 (面板降级空态,
    // 不炸整页); 后端未来新增字段必须被忽略, 不打破现有解码。
    let resp: ListEnvelope<TopupOrderView> =
        serde_json::from_str(r#"{"items":[{"key":"k","futureFlag":true}]}"#).unwrap();
    assert_eq!(resp.items[0].key, "k");
    assert_eq!(resp.items[0].currency, "");
    assert_eq!(resp.items[0].amount, 0);
    assert_eq!(resp.items[0].state, "");
}

#[test]
fn invitees_decodes_camel_case_fields() {
    // 后端 contract::InviteeView 是 camelCase: userKey / name / joinedAt / reward
    // 逐字命中; reward 为 FREE 内部单位 i64 (面板按 fmt_num 展示)。
    let json = r#"{
        "items": [
            {
                "userKey": "usr-aaa",
                "name": "Alice",
                "joinedAt": "2026-08-01T09:00:00Z",
                "reward": 500000
            }
        ]
    }"#;
    let resp: ListEnvelope<InviteeView> = serde_json::from_str(json).unwrap();
    assert_eq!(resp.items.len(), 1);
    let i = &resp.items[0];
    assert_eq!(i.user_key, "usr-aaa");
    assert_eq!(i.name, "Alice");
    assert_eq!(i.joined_at, "2026-08-01T09:00:00Z");
    assert_eq!(i.reward, 500_000);
}

#[test]
fn invitees_empty_items_is_normal_state() {
    // 无人受邀: {"items":[]} 是合法空态, 面板渲染「暂无被邀请用户」。
    let resp: ListEnvelope<InviteeView> = serde_json::from_str(r#"{"items":[]}"#).unwrap();
    assert!(resp.items.is_empty());
}

#[test]
fn invitee_zero_reward_is_valid_row() {
    // 被邀人尚未贡献奖励: reward=0 如实展示 (0 是真实值, 不是字段缺失)。
    let resp: ListEnvelope<InviteeView> = serde_json::from_str(
        r#"{"items":[{"userKey":"u","name":"Bob","joinedAt":"t","reward":0}]}"#,
    )
    .unwrap();
    assert_eq!(resp.items[0].reward, 0);
}

#[test]
fn invitees_multi_rows_keep_order() {
    // 面板逐行渲染, 顺序即后端返回顺序 (后端按 created_at 排序): 乱序会错位。
    let json = r#"{"items":[
        {"userKey":"u1","name":"一","joinedAt":"2026-01-01T00:00:00Z","reward":1},
        {"userKey":"u2","name":"二","joinedAt":"2026-02-01T00:00:00Z","reward":2}
    ]}"#;
    let resp: ListEnvelope<InviteeView> = serde_json::from_str(json).unwrap();
    let names: Vec<_> = resp.items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["一", "二"]);
}
