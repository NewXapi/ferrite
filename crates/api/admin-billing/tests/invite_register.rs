//! 邀请链接闭环（#197）注册接入测试 — 纯逻辑，不需要 PG。
//!
//! 覆盖注册链路最脆弱的两环：
//! - `contract::api::auth::RegisterRequest` 的 wire 契约：带/不带 invite、
//!   camelCase 拼写、null 与 round-trip——前端构造的请求体后端必须原样解析，
//!   `#[serde(default)]` 保证老客户端（自然注册）不断线。
//! - `billing::currency::parse_invite_code` 边界：脏输入（空串/非 UUID/截断）
//!   一律 None，绝不带着垃圾去碰 DB。
//!
//! DB 侧的邀请归属语义（inviter 存在性、先到先得、自邀请拒绝）由
//! `bind_inviter` 自身在 `topup_affiliate.rs` 覆盖，本文件不重复。

use billing::currency::parse_invite_code;
use contract::api::auth::RegisterRequest;
use uuid::Uuid;

#[test]
fn register_request_with_invite_parses_camel_case() {
    // 前端按 camelCase 发 invite（与 username/password/email 同格）。
    let key = Uuid::new_v4();
    let body =
        format!(r#"{{"username":"bob","password":"secret","email":"bob@x.io","invite":"{key}"}}"#);
    let req: RegisterRequest = serde_json::from_str(&body).expect("camelCase invite must parse");

    assert_eq!(req.username, "bob");
    assert_eq!(req.email.as_deref(), Some("bob@x.io"));
    assert_eq!(req.invite, Some(key.to_string()));
}

#[test]
fn register_request_without_invite_defaults_to_none() {
    // 自然注册 / 老客户端不带 invite——#[serde(default)] 兜成 None。
    let body = r#"{"username":"bob","password":"secret"}"#;
    let req: RegisterRequest = serde_json::from_str(body).expect("missing invite must parse");

    assert_eq!(req.invite, None);
    assert_eq!(req.email, None);
}

#[test]
fn register_request_invite_null_parses_to_none() {
    // 前端把 invite 序列化成 null（Option 显式缺省）也得照常解析。
    let body = r#"{"username":"bob","password":"secret","invite":null}"#;
    let req: RegisterRequest = serde_json::from_str(body).expect("null invite must parse");

    assert_eq!(req.invite, None);
}

#[test]
fn register_request_invite_round_trips() {
    // 前端构造 → 序列化 → 后端反序列化必须无损往返。
    let key = Uuid::new_v4();
    let req = RegisterRequest {
        username: "bob".into(),
        password: "secret".into(),
        email: None,
        invite: Some(key.to_string()),
    };
    let json = serde_json::to_string(&req).expect("serialize");
    assert!(
        json.contains(&format!("\"invite\":\"{key}\"")),
        "serialized body must carry invite: {json}"
    );

    let back: RegisterRequest = serde_json::from_str(&json).expect("round-trip");
    assert_eq!(req, back);
}

#[test]
fn parse_invite_code_accepts_valid_uuid() {
    let key = Uuid::new_v4();
    assert_eq!(parse_invite_code(Some(&key.to_string())), Some(key));
}

#[test]
fn parse_invite_code_rejects_dirty_input() {
    // None / 空串 / 非 UUID / 截断 UUID 一律 None：脏输入在碰 DB 前被丢弃。
    assert_eq!(parse_invite_code(None), None);
    assert_eq!(parse_invite_code(Some("")), None);
    assert_eq!(parse_invite_code(Some("not-a-uuid")), None);
    assert_eq!(parse_invite_code(Some("550e8400-e29b-41d4-a716")), None);
}
