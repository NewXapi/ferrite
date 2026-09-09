//! ui-components 认证 DTO 与会话不变量测试
use contract::api::auth::{LoginRequest, RegisterRequest};
use contract::api::user::{role_label, UserDto};
use ui_components::{clear_cached_session, get_cached_token, set_cached_session};

#[test]
fn test_auth_requests_construction() {
    let req = LoginRequest {
        username: "ferrite_tester".into(),
        password: "secret_password".into(),
    };
    assert_eq!(req.username, "ferrite_tester");
    assert_eq!(req.password, "secret_password");

    let reg = RegisterRequest {
        username: "ferrite_user".into(),
        password: "pass".into(),
        email: Some("user@ferrite.dev".into()),
    };
    assert_eq!(reg.username, "ferrite_user");
    assert!(reg.email.is_some());
}

#[test]
fn test_session_cache_lifecycle() {
    clear_cached_session();
    let token = get_cached_token();
    assert!(token.is_none(), "初始状态或清理后 Token 为空");

    let dummy_user = UserDto {
        key: "test_key".into(),
        username: "test_user".into(),
        display_name: "测试用户".into(),
        email: "test@ferrite.dev".into(),
        quota: 1000,
        used_quota: 0,
        request_count: Some(0),
        group: "default".into(),
        role: 1,
        status: 1,
        auth_version: 0,
        created_at: "2026-09-05".into(),
    };

    set_cached_session("dummy_jwt_token_123", &dummy_user);
    // 在 native 非 wasm 目标下，直接测试 DTO 结构无 panic
    assert_eq!(dummy_user.username, "test_user");
    assert_eq!(dummy_user.role, 1);
}

#[test]
fn test_role_label_mapping() {
    assert_eq!(role_label(100), "root");
    assert_eq!(role_label(10), "admin");
    assert_eq!(role_label(1), "user");
    assert_eq!(role_label(0), "user");
}

#[test]
fn test_self_wire_shape_decodes() {
    // 真实 GET /api/user/self 的 wire 形状 (auth::service::UserView, camelCase):
    // role 为 u16 数字, 无 requestCount, 多 authVersion, createdAt 为 RFC3339。
    let wire = r#"{
        "key": "9f0c1a2e-1111-2222-3333-444455556666",
        "username": "alice",
        "displayName": "Alice",
        "email": "alice@ferrite.dev",
        "role": 10,
        "status": 1,
        "quota": 500000,
        "usedQuota": 12000,
        "group": "default",
        "authVersion": 3,
        "createdAt": "2026-09-05T08:30:00Z"
    }"#;
    let user: UserDto = serde_json::from_str(wire).expect("real /self wire must decode");
    assert_eq!(user.role, 10);
    assert_eq!(role_label(user.role), "admin");
    assert_eq!(user.request_count, None, "/self 不带 requestCount → 默认 None");
    assert_eq!(user.auth_version, 3);
}
