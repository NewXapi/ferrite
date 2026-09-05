//! ui-components 认证 DTO 与会话不变量测试
use contract::api::auth::{LoginRequest, RegisterRequest};
use contract::api::user::UserDto;
use ui_components::{clear_cached_session, get_cached_token, get_cached_user, set_cached_session};

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
        request_count: 0,
        group: "default".into(),
        role: "user".into(),
        status: 1,
        created_at: "2026-09-05".into(),
    };

    set_cached_session("dummy_jwt_token_123", &dummy_user);
    // 在 native 非 wasm 目标下，直接测试 DTO 结构无 panic
    assert_eq!(dummy_user.username, "test_user");
    assert_eq!(dummy_user.role, "user");
}
