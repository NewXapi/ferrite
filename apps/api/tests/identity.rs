use api::identity::{require_admin, Pass};
use axum::http::StatusCode;

fn make_pass(is_admin: bool) -> Pass {
    Pass {
        token_key: "test-key".into(),
        user_id: 1,
        username: "test-user".into(),
        quota: 1000,
        used_quota: 0,
        group: "default".into(),
        is_admin,
    }
}

/// admin token → require_admin 返回 Ok
#[test]
fn require_admin_with_admin_returns_ok() {
    let pass = make_pass(true);
    assert!(require_admin(&pass).is_ok());
}

/// 非 admin token → require_admin 返回 403
#[test]
fn require_admin_with_non_admin_returns_forbidden() {
    let pass = make_pass(false);
    let err = require_admin(&pass).unwrap_err();
    assert_eq!(err.0, StatusCode::FORBIDDEN);
}

/// 默认 token 不是 admin
#[test]
fn default_pass_is_not_admin() {
    let pass = make_pass(false);
    assert!(!pass.is_admin);
}

/// admin token 的 is_admin 为 true
#[test]
fn admin_pass_has_is_admin_true() {
    let pass = make_pass(true);
    assert!(pass.is_admin);
}
