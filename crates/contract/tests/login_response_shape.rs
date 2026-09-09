//! 验证契约 DTO 能解析后端真实登录响应。
//!
//! 后端 `UserView` 返回的 `role` 是整数 (1/10/100), 且 `auth_users` 表无
//! `request_count` 列, 因此响应里没有 `requestCount`。前端契约 `UserDto`
//! 历史上要求 `role: String` 且必须有 `requestCount`, 导致
//! `POST /api/user/login` 的响应解析失败 (decode error: invalid type:
//! integer ... expected a string)。本测试用真实抓包 JSON 锁死该契约兼容性。

use contract::api::auth::LoginResponse;

/// 从真实运行后端 (`http://127.0.0.1:3211`) 抓包得到的登录响应 (role=1, 无 requestCount)。
fn real_login_body(role: u32) -> String {
    format!(
        r#"{{
            "accessToken": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.payload.sig",
            "expiresIn": 900,
            "refreshToken": "1009c75c-144d-443f-a8ed-c4ff25812838.f92a4d57fc05ac81b9debcb2af248f85c1dcba8ca74a41a9657970be49584359",
            "user": {{
                "authVersion": 1,
                "createdAt": "2026-09-08T17:55:28.579140Z",
                "displayName": "probe_user",
                "email": "",
                "group": "default",
                "key": "d9334bb6-1652-4e00-ade8-75fe5d5fbc45",
                "quota": 0,
                "role": {role},
                "status": 1,
                "usedQuota": 0,
                "username": "probe_user"
            }}
        }}"#
    )
}

#[test]
fn login_response_parses_with_integer_role_and_missing_request_count() {
    let body = real_login_body(1);
    let resp: LoginResponse =
        serde_json::from_str(&body).expect("后端真实登录响应必须能被 LoginResponse 解析");

    assert_eq!(
        resp.access_token,
        "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.payload.sig"
    );
    assert!(!resp.refresh_token.is_empty());
    assert_eq!(resp.expires_in, 900);

    // role=1 → 语义字符串 "user"; 后端未给 requestCount → 缺省为 0。
    assert_eq!(resp.user.role, "user");
    assert_eq!(resp.user.request_count, 0);
    assert_eq!(resp.user.username, "probe_user");
    assert_eq!(resp.user.status, 1);
}

#[test]
fn login_response_maps_root_role_100_to_semantic_string() {
    let body = real_login_body(100);
    let resp: LoginResponse = serde_json::from_str(&body).expect("role=100 响应必须能解析");
    assert_eq!(resp.user.role, "root");
}

#[test]
fn login_response_maps_admin_role_10_to_semantic_string() {
    let body = real_login_body(10);
    let resp: LoginResponse = serde_json::from_str(&body).expect("role=10 响应必须能解析");
    assert_eq!(resp.user.role, "admin");
}

#[test]
fn login_response_keeps_already_string_role_backwards_compat() {
    let body = real_login_body(1).replace(r#""role": 1"#, r#""role": "user""#);
    let resp: LoginResponse = serde_json::from_str(&body).expect("字符串 role 也应兼容");
    assert_eq!(resp.user.role, "user");
}
