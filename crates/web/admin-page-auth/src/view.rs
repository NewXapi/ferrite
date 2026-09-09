//! Auth page rendering and component composition.
//! Top-anchored layout: logo top-left; tab + title embedded in card.

use crate::api;
use crate::api::contract_auth;
use crate::form::{SignInForm, SignInPayload, SignUpForm, SignUpPayload};
use crate::state::{AuthTab, auth_tab};
use client::ApiClient;
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
            remember: false,
        }
    }
}

use crate::form::SubmitState;

/// 统一提交载荷：register 标志 + 表单字段。
struct SubmitPayload {
    register: bool,
    username: String,
    email: String,
    password: String,
    remember: bool,
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
    let sign_in_class = if is_sign_in {
        "text-zinc-900"
    } else {
        "text-zinc-400 hover:text-zinc-200"
    };
    let register_class = if is_sign_in {
        "text-zinc-400 hover:text-zinc-200"
    } else {
        "text-zinc-900"
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
        state.error.set(None);
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
                    },
                )
                .await
            {
                state.busy.set(false);
                state.error.set(Some(e.to_string()));
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
            class: "relative min-h-screen overflow-x-hidden bg-zinc-950 text-zinc-100",

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
                span { class: "text-lg font-semibold tracking-tight text-zinc-100", "Ferrite" }
                span { class: "text-[10px] font-medium uppercase tracking-widest text-zinc-500", "ADMIN" }
            }

            // Card
            div {
                class: "flex min-h-screen items-center justify-center px-4",
                div {
                    class: "w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900/70 p-8 shadow-2xl shadow-black/40 backdrop-blur",

                    // Tab switcher
                    div {
                        class: "relative mb-6 flex rounded-full border border-zinc-800 bg-zinc-900 p-1",
                        div {
                            class: "absolute inset-y-0 w-1/2 rounded-full bg-zinc-100 transition-transform duration-200 {indicator_transform}",
                        }
                        button {
                            class: "relative z-10 flex-1 rounded-full px-3.5 py-1.5 text-xs font-medium transition-colors duration-200 {sign_in_class}",
                            r#type: "button",
                            role: "tab",
                            aria_selected: "{is_sign_in}",
                            onclick: move |_| crate::state::set_auth_tab(AuthTab::SignIn),
                            "Sign in"
                        }
                        button {
                            class: "relative z-10 flex-1 rounded-full px-3.5 py-1.5 text-xs font-medium transition-colors duration-200 {register_class}",
                            r#type: "button",
                            role: "tab",
                            aria_selected: "{!is_sign_in}",
                            onclick: move |_| crate::state::set_auth_tab(AuthTab::SignUp),
                            "Register"
                        }
                    }

                    h1 { class: "text-xl font-semibold text-zinc-100", "{title_text}" }
                    p { class: "mb-6 text-sm text-zinc-500", "{subtitle_text}" }

                    // Form content
                    match active {
                        AuthTab::SignIn => rsx! { SignInForm { submit: move |p: crate::form::SignInPayload| handle_submit(p.into()), remember: remember_signal } },
                        AuthTab::SignUp => rsx! { SignUpForm { submit: move |p: crate::form::SignUpPayload| handle_submit(p.into()) } },
                    }
                }

                // Footer
                p {
                    class: "mt-6 text-center text-xs text-zinc-600",
                    "By continuing, you agree to our "
                    span {
                        class: "cursor-pointer text-zinc-400 underline underline-offset-2 transition-colors hover:text-zinc-200",
                        "Terms of Service"
                    }
                }
            }
        }
    }
}
