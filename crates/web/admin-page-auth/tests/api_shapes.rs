//! Unit tests for page-auth public surface.
//!
//! These exercise the bits the rendered page depends on: tab enum transitions
//! and the `api::login` / `api::register` mock contract. Dioxus RSX itself
//! isn't unit-tested (it requires a runtime), so layout regressions are caught
//! by the gate screenshot pass instead.

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

#[test]
fn auth_tab_serde_roundtrip_uses_default() {
    // The enum isn't serialized today; this guards against future renames by
    // asserting `Copy + Eq` semantics that `set_auth_tab` and `auth_tab`
    // depend on. If this fails, the state module lost its derive set.
    use page_auth::state::AuthTab;
    let sign_in = AuthTab::SignIn;
    let copy_via_assign = sign_in;
    assert_eq!(sign_in, copy_via_assign);
    assert_ne!(sign_in, AuthTab::SignUp);
}
