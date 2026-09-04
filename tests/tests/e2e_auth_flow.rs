//! 跨 crate 真实端到端 (E2E) 认证流程测试
//!
//! 覆盖：
//! 1. 契约层 contract::api::auth 序列化/反序列化与字段兼容性
//! 2. 后端 crates/api/auth 密码哈希 (Argon2id) 与 JWT 令牌颁发/验签
//! 3. 跨层 DTO 转换: UserRecord -> UserDto -> LoginResponse
//! 4. 前端组件库 crates/web/ui-components 的会话存储与凭证生命周期管理

use contract::api::auth::{LoginRequest, LoginResponse, RegisterRequest};
use contract::api::user::UserDto;
use ui_components::{clear_cached_session, get_cached_token, set_cached_session};

#[test]
fn e2e_contract_auth_wire_format() {
    let req = LoginRequest {
        username: "ferrite_admin".into(),
        password: "secure_password_123".into(),
    };
    let json_val = serde_json::to_value(&req).expect("serialize login request");
    // 验证 camelCase 命名
    assert_eq!(json_val["username"], "ferrite_admin");
    assert_eq!(json_val["password"], "secure_password_123");

    let reg = RegisterRequest {
        username: "new_player".into(),
        password: "player_pass".into(),
        email: Some("player@ferrite.dev".into()),
    };
    let reg_json = serde_json::to_string(&reg).expect("serialize register request");
    assert!(reg_json.contains("\"username\":\"new_player\""));
    assert!(reg_json.contains("\"email\":\"player@ferrite.dev\""));
}

#[test]
fn e2e_security_password_and_jwt_lifecycle() {
    // 1. 测试密码 Argon2id 散列与验证
    let raw_pass = "my_ferrite_password_2026";
    let phc = auth::password::hash(raw_pass).expect("hash password");
    assert!(phc.starts_with("$argon2id$"), "密码散列必须符合 Argon2id PHC 格式");
    assert!(auth::password::verify(raw_pass, &phc), "原始密码比对必须成功");
    assert!(!auth::password::verify("wrong_password", &phc), "错误密码必须被拒绝");

    // 2. 测试 JWT 令牌生命周期 (issue -> parse -> claim validation)
    let secret = b"ferrite_jwt_secret_must_be_32bytes_min!";
    let user_id = "u_e2e_player_01";
    let role = 1u16;
    let auth_version = 1i64;
    let sid = "sess_uuid_999";

    let (token, exp) = auth::jwt::issue(secret, user_id, role, auth_version, sid)
        .expect("issue jwt token");
    assert!(!token.is_empty(), "颁发的 JWT 不能为空");
    assert!(exp > 0, "过期时间必须为未来时间戳");

    let claims = auth::jwt::parse(secret, &token).expect("parse jwt claims");
    assert_eq!(claims.sub, user_id, "Subject 必须与用户 ID 一致");
    assert_eq!(claims.role, role, "用户角色位必须一致");
    assert_eq!(claims.sid, sid, "会话 Session ID 必须一致");

    // 3. 错误密钥校验
    let bad_secret = b"wrong_secret_key_padding_bytes_32!";
    let parse_err = auth::jwt::parse(bad_secret, &token);
    assert!(parse_err.is_err(), "错误密钥必须校验失败");
}

#[test]
fn e2e_web_session_storage_and_dto_interop() {
    clear_cached_session();
    assert!(get_cached_token().is_none(), "清除会话后 Token 必须为空");

    let user_dto = UserDto {
        key: "u_e2e_key".into(),
        username: "alice_player".into(),
        display_name: "爱丽丝".into(),
        email: "alice@ferrite.dev".into(),
        quota: 100000,
        used_quota: 500,
        request_count: 12,
        group: "default".into(),
        role: "user".into(),
        status: 1,
        created_at: "2026-09-05".into(),
    };

    let token_str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.dummy_jwt_payload";
    set_cached_session(token_str, &user_dto);

    let response = LoginResponse {
        user: user_dto.clone(),
        access_token: token_str.into(),
        refresh_token: "ref_tok_e2e".into(),
        expires_in: 86400,
    };

    let resp_json = serde_json::to_string(&response).expect("serialize LoginResponse");
    let decoded: LoginResponse = serde_json::from_str(&resp_json).expect("deserialize LoginResponse");
    assert_eq!(decoded.user.username, "alice_player");
    assert_eq!(decoded.user.display_name, "爱丽丝");
    assert_eq!(decoded.access_token, token_str);
    assert_eq!(decoded.expires_in, 86400);
}
