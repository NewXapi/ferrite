//! 提交载荷映射契约：两个表单的 payload → 统一的 `SubmitPayload`。
//!
//! 关键不变量是 `remember` 必须透传。曾经 `From<SignUpPayload>` 把它硬编码成
//! `false`，注册表单也没有勾选框，于是注册拿到的 token 全进 sessionStorage，
//! 一关浏览器登录态就丢 —— 且编译期毫无提示。这里钉住透传，防止再被常量遮住。
//!
//! 只测纯映射；rsx 里的 `busy` 接线与 Remember me 渲染需要 dioxus 运行时，
//! 由 gate 的浏览器目检覆盖。

use admin_page_auth::components::auth_form::{SignInPayload, SignUpPayload};
use admin_page_auth::components::auth_view::SubmitPayload;

/// 回归护栏：注册勾了 Remember me，`remember` 必须是 true。
#[test]
fn sign_up_passes_remember_through() {
    let p = SubmitPayload::from(SignUpPayload {
        username: "alice".into(),
        email: "alice@example.com".into(),
        password: "secret".into(),
        remember: true,
    });
    assert!(p.register, "注册路径必须带 register 标志");
    assert!(
        p.remember,
        "SignUpPayload.remember 必须透传，不得硬编码 false"
    );
    assert_eq!(p.username, "alice");
    assert_eq!(p.email, "alice@example.com");
    assert_eq!(p.password, "secret");
}

/// 未勾选时不得持久化（token 应落 sessionStorage，关浏览器即失效）。
#[test]
fn sign_up_without_remember_stays_false() {
    let p = SubmitPayload::from(SignUpPayload {
        username: "alice".into(),
        email: String::new(),
        password: "secret".into(),
        remember: false,
    });
    assert!(p.register);
    assert!(!p.remember, "未勾选时 remember 必须保持 false");
}

/// 登录路径本来就透传 `remember`，一并钉住，避免映射被改坏。
#[test]
fn sign_in_passes_remember_through() {
    let p = SubmitPayload::from(SignInPayload {
        username: "alice".into(),
        password: "secret".into(),
        remember: true,
    });
    assert!(!p.register, "登录路径不得带 register 标志");
    assert!(p.remember, "SignInPayload.remember 必须透传");
    assert_eq!(p.username, "alice");
    assert_eq!(p.password, "secret");
    assert!(p.email.is_empty(), "登录不带 email");
}
