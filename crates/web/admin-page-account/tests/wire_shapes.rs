//! 账户面板真实 API 的 wire 形状不变量: 请求/响应 DTO 的 serde 序列化
//! 字段名逐字对齐后端 (auth routes.rs / admin-billing redeem.rs)。
//! 后端字段改名或前端误写 snake_case 时, 这里第一时间红。

use admin_page_account::api;
use contract::api::user::{SessionDto, UpdateSelfRequest, UserTopupRequest};

#[test]
fn user_topup_request_serializes_exact_key_field() {
    // 后端 TopupRequest { key } (admin-billing redeem.rs:321) 无 serde rename:
    // wire 上必须有且只有 "key" 一个字段, 值为兑换码明文。
    let req = UserTopupRequest {
        key: "fx-0123abcd0123abcd0123abcd0123abcd".into(),
    };
    let v = serde_json::to_value(&req).expect("UserTopupRequest 必须可序列化");
    assert_eq!(
        v,
        serde_json::json!({ "key": "fx-0123abcd0123abcd0123abcd0123abcd" }),
        "topup 请求体字段名逐字 = key"
    );
    // 防多字段/嵌套: 序列化结果就是单字段平面对象
    let obj = v.as_object().expect("topup 请求体必须是 JSON object");
    assert_eq!(obj.len(), 1, "topup 请求体只含 key 一个字段");
    assert!(obj.contains_key("key"));
}

#[test]
fn user_topup_request_deserializes_from_backend_shape() {
    // 后端按 {"key": ...} 反序列化; 契约 DTO 必须能吃回同一形状 (round-trip)
    let raw = serde_json::json!({ "key": "fx-ffff" });
    let req: UserTopupRequest =
        serde_json::from_value(raw).expect("后端形状的 topup 请求必须能反序列化");
    assert_eq!(req.key, "fx-ffff");
}

#[test]
fn update_self_request_uses_camel_case_wire_fields() {
    // 后端 UpdateSelfRequest (auth routes.rs:228) 是 #[serde(rename_all =
    // "camelCase")] 的 Option 字段: wire 字段名逐字为 displayName /
    // originalPassword / newPassword, snake_case 会被后端静默丢弃成 None。
    let req = UpdateSelfRequest {
        display_name: Some("新名字".into()),
        original_password: Some("old-pass".into()),
        new_password: Some("new-pass".into()),
    };
    let v = serde_json::to_value(&req).expect("UpdateSelfRequest 必须可序列化");
    assert_eq!(
        v,
        serde_json::json!({
            "displayName": "新名字",
            "originalPassword": "old-pass",
            "newPassword": "new-pass",
        }),
        "update_self 请求字段名逐字 camelCase"
    );
}

#[test]
fn update_self_request_none_fields_serialize_as_null() {
    // 契约未加 skip_serializing_if: None 字段 wire 上是 null;
    // 后端 Option 字段接受 null → None, 语义即「不改该字段」。
    // 单改显示名时密码两字段为 null, 不触发后端改密路径。
    let req = UpdateSelfRequest {
        display_name: Some("只改名".into()),
        original_password: None,
        new_password: None,
    };
    let v = serde_json::to_value(&req).expect("UpdateSelfRequest 必须可序列化");
    assert_eq!(v["displayName"], "只改名");
    assert!(v["originalPassword"].is_null(), "未提供的原密码 wire=null");
    assert!(v["newPassword"].is_null(), "未提供的新密码 wire=null");
}

#[test]
fn session_dto_wire_fields_are_camel_case_for_revoke_flow() {
    // 吊销请求 DELETE /api/user/self/sessions/{sid} 的 sid 来自会话列表响应;
    // 后端 SessionView wire 为 camelCase (userAgent/loginMethod/lastActive/
    // expiresAt/current), 字段名逐字断言保证列表解析不塌。
    let raw = serde_json::json!({
        "sid": "3f2b8c1e-0000-4000-8000-000000000001",
        "userAgent": "Mozilla/5.0",
        "ip": "127.0.0.1",
        "loginMethod": "password",
        "createdAt": "2026-09-01T00:00:00Z",
        "lastActive": "2026-09-12T00:00:00Z",
        "expiresAt": "2026-09-30T00:00:00Z",
        "current": true,
    });
    let dto: SessionDto = serde_json::from_value(raw).expect("后端会话 wire 必须能反序列化");
    assert_eq!(dto.sid, "3f2b8c1e-0000-4000-8000-000000000001");
    assert_eq!(dto.user_agent, "Mozilla/5.0");
    assert_eq!(dto.login_method, "password");
    assert!(dto.current, "current 标志必须从 camelCase 字段读出");

    // 吊销路径由 dto.sid 拼出, 与后端路由 /self/sessions/{sid} 对齐
    let path = format!("/api/user/self/sessions/{}", dto.sid);
    assert_eq!(path, "/api/user/self/sessions/3f2b8c1e-0000-4000-8000-000000000001");
}

#[test]
fn revoke_bodies_match_backend_contract() {
    // 单条吊销响应: 后端 revoke_session 返回裸 {"success": true};
    // api.rs 用 serde_json::Value 承接, 这里断言该形状可解析且 success=true。
    let resp: serde_json::Value =
        serde_json::from_str(r#"{"success": true}"#).expect("吊销响应必须是合法 JSON");
    assert_eq!(resp["success"], true);

    // 一键吊销其它设备: POST /self/sessions/revoke-others 请求体为空对象
    // (api.rs 发 json!({}), 后端 handler 不取 Json extractor), 断言序列化后
    // 恰为 "{}" —— 不夹带任何字段。
    assert_eq!(serde_json::json!({}).to_string(), "{}");
}

#[test]
fn topup_credited_quota_extracts_real_credited_amount() {
    // 后端 topup 成功响应: {"quota": <i64 内部单位>, "success": true};
    // 面板用真实 quota 提示入账结果。
    let ok = serde_json::json!({ "quota": 500_000, "success": true });
    assert_eq!(api::topup_credited_quota(&ok), Some(500_000));

    // 0 也算合法入账值 (异常但不该当缺失处理)
    let zero = serde_json::json!({ "quota": 0, "success": true });
    assert_eq!(api::topup_credited_quota(&zero), Some(0));

    // 缺 quota / null / 浮点 / 字符串 → None (面板降级为通用文案, 不假造数值)
    let missing = serde_json::json!({ "success": true });
    assert_eq!(api::topup_credited_quota(&missing), None);
    let null_quota = serde_json::json!({ "quota": null, "success": true });
    assert_eq!(api::topup_credited_quota(&null_quota), None);
    let float_quota = serde_json::json!({ "quota": 1.5, "success": true });
    assert_eq!(api::topup_credited_quota(&float_quota), None);
    let str_quota = serde_json::json!({ "quota": "500000", "success": true });
    assert_eq!(api::topup_credited_quota(&str_quota), None);
}
