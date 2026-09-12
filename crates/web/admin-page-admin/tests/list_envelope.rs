//! 列表信封 wire 契约测试。
//!
//! 后端 admin-catalog 的列表端点 `GET /api/group`（groups.rs `list`）、
//! `GET /api/channel`（channels.rs `list`）、`GET /api/token`（tokens.rs `list`）
//! 的响应 data 都是 `{"items":[...]}`（渠道另有 `total`）。而历史上
//! `api.rs` 的列表 helper 按裸 `Vec<Dto>` 解码，`ApiClient` 剥掉最外层
//! （若有）Envelope 后拿到的仍是 map，运行时实锤报错：
//! `decode error: invalid type: map, expected a sequence at line 1 column 0`。
//!
//! 本文件双向钉死这一现场：
//! - 正向：真实 wire 样例经剥壳逻辑（[`Items`] / 契约 [`TokenList`]）可解出预期 Vec；
//! - 反向：同一 map 按裸 `Vec<Dto>` 解码必须失败，且错误消息与线上事故一致
//!   （防止有人把 helper 签名改回裸 Vec 而 CI 无感）。

use admin_page_admin::api::Items;
use contract::api::admin::{ChannelDto, GroupDto};
use contract::api::token::{TokenDto, TokenList};

/// GET /api/group 的真实 wire（字段对齐 admin-catalog `GroupView`，camelCase）。
const GROUP_WIRE: &str = r#"{"items":[
  {"key":"3f2a8b1e-0000-0000-0000-000000000001","name":"default","ratio":1.0,
   "modelWhitelist":[],"remark":"默认分组","status":1,
   "createdAt":"2026-08-30T00:00:00Z","updatedAt":"2026-08-30T00:00:00Z"},
  {"key":"9c14d2a2-0000-0000-0000-000000000002","name":"vip","ratio":0.8,
   "modelWhitelist":null,"remark":"","status":1,
   "createdAt":"2026-09-01T00:00:00Z","updatedAt":"2026-09-01T00:00:00Z"}
]}"#;

/// GET /api/channel 的真实 wire：比分组多一个 `total`（列表 helper 不消费它，
/// `Items<T>` 未声明该字段即自然忽略——这条样例同时钉住「多余字段不炸」）。
const CHANNEL_WIRE: &str = r#"{"total":42,"items":[
  {"key":"c1","name":"OpenAI 官方","channelType":"openai",
   "baseUrl":"https://api.openai.com/v1","keyCount":2,"keys":null,
   "models":["gpt-4o"],"groups":["default"],"priority":0,"weight":10,"status":1,
   "tags":[],"testModel":"gpt-4o","remark":"",
   "createdAt":"2026-08-30T00:00:00Z","updatedAt":"2026-08-30T00:00:00Z"}
]}"#;

/// GET /api/token 的真实 wire（字段对齐 admin-catalog TokenView，camelCase）。
const TOKEN_WIRE: &str = r#"{"items":[
  {"key":"t1","userKey":"u1","name":"ci","plainKey":null,"keyPreview":"sk-ab****ef",
   "group":null,"quota":500000,"unlimitedQuota":false,"usedQuota":0,"status":1,
   "expiresAt":null,"createdAt":"2026-08-30T00:00:00Z"}
]}"#;

/// 正向：分组列表 `{items}` 信封剥壳后得到两条 GroupDto，字段逐一符合预期。
#[test]
fn group_items_envelope_unwraps_to_vec() {
    let r: Items<GroupDto> = serde_json::from_str(GROUP_WIRE).expect("分组 wire 应可剥壳解码");
    assert_eq!(r.items.len(), 2, "两条分组");
    assert_eq!(r.items[0].name, "default");
    assert_eq!(r.items[0].ratio, 1.0);
    assert_eq!(r.items[0].remark, "默认分组");
    assert_eq!(r.items[0].model_whitelist, serde_json::json!([]));
    assert_eq!(r.items[1].name, "vip");
    assert_eq!(r.items[1].ratio, 0.8);
    assert_eq!(r.items[1].model_whitelist, serde_json::Value::Null);
}

/// 正向：渠道列表除 items 外另有 total，剥壳逻辑忽略它且不得报错。
#[test]
fn channel_items_envelope_unwraps_and_ignores_total() {
    let r: Items<ChannelDto> =
        serde_json::from_str(CHANNEL_WIRE).expect("渠道 wire（含 total）应可剥壳解码");
    assert_eq!(r.items.len(), 1);
    let c = &r.items[0];
    assert_eq!(c.key, "c1");
    assert_eq!(c.name, "OpenAI 官方");
    assert_eq!(c.channel_type, "openai");
    assert_eq!(c.base_url, "https://api.openai.com/v1");
    assert_eq!(c.key_count, 2);
    assert_eq!(c.status, 1);
    assert_eq!(c.groups, vec!["default".to_string()]);
    assert_eq!(c.models, serde_json::json!(["gpt-4o"]));
}

/// 正向：令牌列表用契约层 `TokenList`（token.rs 即为此信封而生）剥壳。
#[test]
fn token_items_envelope_unwraps_to_vec() {
    let r: TokenList = serde_json::from_str(TOKEN_WIRE).expect("令牌 wire 应可剥壳解码");
    assert_eq!(r.items.len(), 1);
    let t: &TokenDto = &r.items[0];
    assert_eq!(t.key, "t1");
    assert_eq!(t.name, "ci");
    assert_eq!(t.key_preview, "sk-ab****ef");
    assert_eq!(t.quota, 500000);
    assert_eq!(t.status, 1);
    assert_eq!(t.plain_key, None, "列表里明文恒不下发");
}

/// 边界：`items` 为空数组或缺失（后端理论上恒带 items，防御性钉住
/// `#[serde(default)]` 行为），剥壳得到空 Vec 而非报错。
#[test]
fn items_envelope_tolerates_empty_and_missing_items() {
    let empty: Items<GroupDto> =
        serde_json::from_str(r#"{"items":[]}"#).expect("空 items 应解出空 Vec");
    assert!(empty.items.is_empty());
    let missing: Items<ChannelDto> =
        serde_json::from_str("{}").expect("缺 items 应按 default 解出空 Vec");
    assert!(missing.items.is_empty());
}

/// 反向（钉死历史 bug 现场）：同一 `{items}` map 按裸 `Vec<Dto>` 解码必须失败，
/// 且错误消息与浏览器里实测的 `invalid type: map, expected a sequence at line 1
/// column 0` 一致。列表端点若哪天被改回真数组，这里会以「解出成功」的方式报警。
#[test]
fn bare_vec_decode_rejects_items_map() {
    assert_bare_vec_rejects::<GroupDto>("group", GROUP_WIRE);
    assert_bare_vec_rejects::<ChannelDto>("channel", CHANNEL_WIRE);
    assert_bare_vec_rejects::<TokenDto>("token", TOKEN_WIRE);
}

fn assert_bare_vec_rejects<T: serde::de::DeserializeOwned + core::fmt::Debug>(
    label: &str,
    wire: &str,
) {
    let err = serde_json::from_str::<Vec<T>>(wire)
        .expect_err(&format!("{label}: {{items}} 信封按裸 Vec 解码必须失败"));
    assert!(
        err.to_string()
            .contains("invalid type: map, expected a sequence"),
        "{label}: 错误消息应复现线上 decode error，实际为 {err}"
    );
}
