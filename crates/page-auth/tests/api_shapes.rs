//! auth 集成测试: login/register mock 契约。
//! 接真后端时这里换成 HTTP 集成测试, 契约 (LoginResponse.token 非空) 不变。

use page_auth::api::{LoginRequest, login, register};

#[tokio::test]
async fn login_returns_nonempty_token() {
    let resp = login(LoginRequest {
        username: "alice".into(),
        password: "secret".into(),
    })
    .await
    .expect("mock login 应成功");
    assert!(!resp.token.is_empty(), "token 非空");
}

#[tokio::test]
async fn register_returns_nonempty_token() {
    let resp = register("bob".into(), "secret".into())
        .await
        .expect("mock register 应成功");
    assert!(!resp.token.is_empty(), "token 非空");
}
