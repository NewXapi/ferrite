//! AuthState token / 未授权回调状态机(从 src/manage_auth_token.rs 内联测试迁出)。
//! `AuthState` 本是 crate 内部件;为满足"测试统一放 tests/"的项目约定以
//! `#[doc(hidden)] pub` 暴露,非公共 API,勿在 crate 之外使用。

use client::AuthState;
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn token_roundtrip() {
    let mut state = AuthState::new();
    assert_eq!(state.token(), None);
    state.set_token(Some("abc".to_string()));
    assert_eq!(state.token(), Some("abc".to_string()));
    state.set_token(None);
    assert_eq!(state.token(), None);
}

#[test]
fn fire_unauthorized_calls_hook() {
    let fired = Rc::new(Cell::new(false));
    let mut state = AuthState::new();
    let fired_clone = fired.clone();
    state.set_on_unauthorized(Box::new(move || fired_clone.set(true)));
    assert!(!fired.get());
    state.fire_unauthorized();
    assert!(fired.get());
}
