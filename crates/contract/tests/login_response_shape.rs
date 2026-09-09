//! 登录响应形状回归测试（真实抓包 JSON,2026-09-09 smoke）。
//!
//! 后端 UserView.role 以整数 (1/10/100) 返回且不含 requestCount；
//! 契约 UserDto.role 是语义字符串 —— 两种形状都必须能解码。

use contract::api::auth::LoginResponse;

/// 真实登录响应（root 账号,ui_smoke_0909）,字段名与缺列保持原样。
const REAL_LOGIN_ROOT: &str = r#"{
  "accessToken": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.x.y",
  "expiresIn": 900,
  "refreshToken": "9a232f19-a1c8-4ad4-b1e2-69a4f3152f2d.deadbeef",
  "user": {
    "authVersion": 1,
    "createdAt": "2026-09-09T10:42:44.882652Z",
    "displayName": "ui_smoke_0909",
    "email": "",
    "group": "default",
    "key": "7dec7cf3-9b50-4b37-b125-be77015fbbe4",
    "quota": 0,
    "role": 100,
    "status": 1,
    "usedQuota": 0,
    "username": "ui_smoke_0909"
  }
}"#;

#[test]
fn real_login_response_with_integer_role_decodes() {
    let resp: LoginResponse = serde_json::from_str(REAL_LOGIN_ROOT)
        .expect("真实登录响应必须能解码（整数 role + 缺 requestCount）");
    assert_eq!(resp.user.role, "root", "整数 100 归一为 root");
    assert_eq!(resp.user.request_count, 0, "缺省 requestCount = 0");
    assert_eq!(resp.access_token.split('.').count(), 3);
    assert_eq!(resp.expires_in, 900);
}

#[test]
fn admin_and_plain_integer_roles_normalize() {
    let user = r#"{
        "key": "k", "username": "u", "displayName": "u", "email": "",
        "quota": 0, "usedQuota": 0, "group": "default",
        "role": 10, "status": 1, "createdAt": "2026-01-01T00:00:00Z"
    }"#;
    let dto: contract::api::user::UserDto = serde_json::from_str(user).unwrap();
    assert_eq!(dto.role, "admin");

    let user = user.replace("\"role\": 10", "\"role\": 1");
    let dto: contract::api::user::UserDto = serde_json::from_str(&user).unwrap();
    assert_eq!(dto.role, "user");
}

#[test]
fn legacy_string_role_still_accepted() {
    let user = r#"{
        "key": "k", "username": "u", "displayName": "u", "email": "",
        "quota": 0, "usedQuota": 0, "requestCount": 7, "group": "default",
        "role": "admin", "status": 1, "createdAt": "2026-01-01"
    }"#;
    let dto: contract::api::user::UserDto = serde_json::from_str(user).unwrap();
    assert_eq!(dto.role, "admin", "历史字符串 role 兼容");
    assert_eq!(dto.request_count, 7);
}
