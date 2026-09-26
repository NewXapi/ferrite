//! Auth page rendering and component composition.
//! Top-anchored layout: logo top-left; tab + title embedded in card.

use crate::api;
use crate::api::contract_auth;
use crate::components::auth_form::{SignInForm, SignInPayload, SignUpForm, SignUpPayload};
use crate::components::auth_state::{AuthTab, auth_tab};
use client::{ApiClient, ApiError};
use dioxus::prelude::*;

impl From<SignInPayload> for SubmitPayload {
    fn from(p: SignInPayload) -> Self {
        Self {
            register: false,
            username: p.username,
            email: String::new(),
            password: p.password,
            remember: p.remember,
        }
    }
}

impl From<SignUpPayload> for SubmitPayload {
    fn from(p: SignUpPayload) -> Self {
        Self {
            register: true,
            username: p.username,
            email: p.email,
            password: p.password,
            remember: p.remember,
        }
    }
}

use crate::components::auth_form::SubmitState;

/// 统一提交载荷：register 标志 + 表单字段。
///
/// 公开是为了让 `tests/submit_payload.rs` 能钉住两个表单的 `From` 映射 ——
/// `remember` 必须透传。曾经 `From<SignUpPayload>` 把它硬编码成 `false`，
/// 注册表单也没有勾选框，注册拿到的 token 全进 sessionStorage，一关浏览器
/// 登录态就丢，且编译期毫无提示。
pub struct SubmitPayload {
    pub register: bool,
    pub username: String,
    pub email: String,
    pub password: String,
    pub remember: bool,
}

/// 当前页面 query 串 (`?invite=…`),无 window / 非 wasm 时返回空串。
fn search_query() -> String {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default()
}

#[component]
pub fn AuthPage() -> Element {
    let active = auth_tab();
    let is_sign_in = active == AuthTab::SignIn;

    let remember_signal = use_signal(|| false);
    let mut state = use_context::<SubmitState>();

    let indicator_transform = if is_sign_in {
        "translate-x-0"
    } else {
        "translate-x-full"
    };
    let indicator_class = format!("{} {}", ui::TAB_INDICATOR, indicator_transform);
    let sign_in_class = if is_sign_in {
        ui::TAB_ACTIVE
    } else {
        ui::TAB_INACTIVE
    };
    let register_class = if is_sign_in {
        ui::TAB_INACTIVE
    } else {
        ui::TAB_ACTIVE
    };
    let title_text = if is_sign_in {
        "Welcome back"
    } else {
        "Create your account"
    };
    let subtitle_text = if is_sign_in {
        "Sign in to continue to Ferrite"
    } else {
        "Get started with Ferrite in seconds"
    };

    // 提交处理：登录或注册（wasm 真调用），成功 → set_token + 回 console。
    let mut handle_submit = move |payload: SubmitPayload| {
        state.busy.set(true);
        // 注册时透传 URL ?invite=<inviter user_key> (邀请链接落地页),
        // 后端 OnUserRegistered 建立归属; 解析失败按无邀请码继续, 不阻断注册。
        let invite = if payload.register {
            api::parse_invite_query(&search_query())
        } else {
            None
        };
        spawn(async move {
            let client = ApiClient::shared().clone();
            // 注册成功后自动登录拿 access_token（register 本身只回 SelfView）
            if payload.register
                && let Err(e) = api::register_api(
                    &client,
                    &contract_auth::RegisterRequest {
                        username: payload.username.clone(),
                        password: payload.password.clone(),
                        email: (!payload.email.is_empty()).then(|| payload.email.clone()),
                        invite: invite.clone(),
                    },
                )
                .await
            {
                state.busy.set(false);
                // 后端对重复用户名/邮箱返回 409；翻译成中文，避免裸 `HTTP 409: Conflict`。
                let msg = match &e {
                    ApiError::Http { status, .. } if *status == 409 => {
                        "该用户名或邮箱已被占用，请换一个再试".to_string()
                    }
                    _ => e.to_string(),
                };
                state.error.set(Some(msg));
                return;
            }
            let result = api::login_api(
                &client,
                &contract_auth::LoginRequest {
                    username: payload.username.clone(),
                    password: payload.password.clone(),
                },
            )
            .await
            .map(|resp| (resp.access_token, resp.refresh_token));
            match result {
                Ok((access_token, refresh_token)) => {
                    client.set_token(Some(access_token.clone()));
                    // Remember me：token 持久化到 localStorage（30 天长效，配 refresh 静默续期）；
                    // 未勾选：sessionStorage（关浏览器即失效）。
                    // ponytail: refresh 流程接入时统一走 admin-session
                    ui::set_storage_scoped("ferrite_access_token", &access_token, payload.remember);
                    ui::set_storage_scoped(
                        "ferrite_refresh_token",
                        &refresh_token,
                        payload.remember,
                    );
                    ui::set_storage_scoped("ferrite_username", &payload.username, payload.remember);
                    state.busy.set(false);
                    // 回控制台：auth hash 清掉，HomePage 重新挂载
                    if let Some(w) = web_sys::window() {
                        let _ = w.location().set_hash("");
                    }
                }
                Err(e) => {
                    state.busy.set(false);
                    state.error.set(Some(e.to_string()));
                }
            }
        });
    };

    rsx! {
        div {
            class: ui::PAGE_BG,

            // Background: dim grid (hairline)
            svg {
                class: "absolute inset-0 h-full w-full",
                xmlns: "http://www.w3.org/2000/svg",
                defs {
                    pattern {
                        id: "grid",
                        width: "80",
                        height: "80",
                        pattern_units: "userSpaceOnUse",
                        path {
                            d: "M 80 0 L 0 0 0 80",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "0.5",
                            class: "text-zinc-800/50",
                        }
                    }
                }
                rect { width: "100%", height: "100%", fill: "url(#grid)" }
            }

            // Logo
            div {
                class: "absolute top-6 left-8 flex items-center gap-2",
                span { class: "{ui::TYPE_TITLE} tracking-tight", "Ferrite" }
                span { class: "{ui::TYPE_LABEL} uppercase tracking-widest", "ADMIN" }
            }

            // Card
            div {
                class: "flex min-h-screen items-center justify-center px-4",
                div {
                    class: ui::AUTH_CARD,

                    // Tab switcher
                    div {
                        class: ui::TAB_SWITCHER,
                        div {
                            class: indicator_class,
                        }
                        button {
                            class: sign_in_class,
                            r#type: "button",
                            role: "tab",
                            aria_selected: "{is_sign_in}",
                            onclick: move |_| crate::components::auth_state::set_auth_tab(AuthTab::SignIn),
                            "Sign in"
                        }
                        button {
                            class: register_class,
                            r#type: "button",
                            role: "tab",
                            aria_selected: "{!is_sign_in}",
                            onclick: move |_| crate::components::auth_state::set_auth_tab(AuthTab::SignUp),
                            "Register"
                        }
                    }

                    h1 { class: "{ui::TYPE_VALUE}", "{title_text}" }
                    p { class: "mb-6 {ui::TYPE_BODY}", "{subtitle_text}" }

                    // Form content
                    match active {
                        AuthTab::SignIn => rsx! { SignInForm { submit: move |p: crate::components::auth_form::SignInPayload| handle_submit(p.into()), remember: remember_signal } },
                        AuthTab::SignUp => rsx! { SignUpForm { submit: move |p: crate::components::auth_form::SignUpPayload| handle_submit(p.into()), remember: remember_signal } },
                    }

                    // Footer — placed clearly BELOW the form, inside the card, separated by a hairline.
                    p {
                        class: "mt-6 border-t border-zinc-800 pt-4 text-center {ui::TYPE_DESC}",
                        "By continuing, you agree to our "
                        span {
                            class: "cursor-pointer {ui::C_MUTED} underline underline-offset-2 transition-colors hover:text-zinc-200",
                            "Terms of Service"
                        }
                    }
                }
            }
        }
    }
}
