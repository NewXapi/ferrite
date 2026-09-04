//! 共享登录/注册弹窗组件 (AuthModal) 与用户态徽标 (UserBadge)。

use dioxus::prelude::*;
use contract::api::auth::{LoginRequest, RegisterRequest};
use contract::api::user::UserDto;

use crate::session::{api_login, api_register, clear_cached_session, get_cached_user};

/// 认证弹窗: 登录 / 注册双 Tab 切换，景深磨砂玻璃拟态
#[component]
pub fn AuthModal(
    open: bool,
    on_close: EventHandler<()>,
    on_success: EventHandler<UserDto>,
) -> Element {
    if !open {
        return rsx! {};
    }

    let mut is_register = use_signal(|| false);
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut email = use_signal(String::new);

    let mut error_msg = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);

    let submit = move |_| {
        let u = username().trim().to_string();
        let p = password().trim().to_string();
        let em = email().trim().to_string();

        if u.is_empty() || p.is_empty() {
            error_msg.set(Some("请输入用户名和密码".into()));
            return;
        }

        loading.set(true);
        error_msg.set(None);

        let is_reg = is_register();
        spawn(async move {
            let res = if is_reg {
                api_register(RegisterRequest {
                    username: u,
                    password: p,
                    email: if em.is_empty() { None } else { Some(em) },
                }).await
            } else {
                api_login(LoginRequest {
                    username: u,
                    password: p,
                }).await
            };

            loading.set(false);
            match res {
                Ok(resp) => {
                    on_success.call(resp.user);
                    on_close.call(());
                }
                Err(err) => {
                    error_msg.set(Some(err));
                }
            }
        });
    };

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-xl p-4 select-none animate-in fade-in duration-200",
            onclick: move |_| on_close.call(()),
            div {
                class: "relative flex w-full max-w-sm flex-col gap-5 rounded-3xl border border-purple-500/40 bg-gradient-to-b from-zinc-900/95 via-zinc-950/95 to-black p-6 sm:p-7 shadow-2xl shadow-purple-950/40 text-xs text-zinc-100",
                onclick: move |e| e.stop_propagation(),

                // 顶部标题与关闭
                div { class: "flex items-center justify-between border-b border-zinc-800/80 pb-3",
                    div { class: "flex items-center gap-2",
                        span { class: "text-base", "🔑" }
                        span { class: "font-serif text-sm font-bold text-white",
                            if is_register() { "加入 Tavern · 账号注册" } else { "登录 Tavern 平台" }
                        }
                    }
                    button {
                        class: "text-zinc-500 hover:text-white transition-colors text-sm",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                // 登录 / 注册 Tab 切换
                div { class: "grid grid-cols-2 gap-1 rounded-xl bg-zinc-950 p-1 border border-zinc-800",
                    button {
                        class: if !is_register() {
                            "rounded-lg bg-zinc-800 py-1.5 font-bold text-white shadow-sm transition-all"
                        } else {
                            "rounded-lg py-1.5 font-medium text-zinc-400 hover:text-zinc-200 transition-colors"
                        },
                        onclick: move |_| {
                            is_register.set(false);
                            error_msg.set(None);
                        },
                        "账号登录"
                    }
                    button {
                        class: if is_register() {
                            "rounded-lg bg-zinc-800 py-1.5 font-bold text-white shadow-sm transition-all"
                        } else {
                            "rounded-lg py-1.5 font-medium text-zinc-400 hover:text-zinc-200 transition-colors"
                        },
                        onclick: move |_| {
                            is_register.set(true);
                            error_msg.set(None);
                        },
                        "注册新账号"
                    }
                }

                // 错误提示条
                if let Some(err) = error_msg() {
                    div { class: "flex items-center gap-2 rounded-xl border border-rose-500/40 bg-rose-950/30 p-2.5 text-[11px] text-rose-300",
                        span { "⚠️" }
                        span { "{err}" }
                    }
                }

                // 表单字段
                div { class: "space-y-3.5",
                    div { class: "flex flex-col gap-1.5",
                        span { class: "font-medium text-zinc-400", "用户名" }
                        input {
                            class: "w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3.5 py-2.5 text-zinc-100 outline-none focus:border-purple-500 transition-colors",
                            placeholder: "输入您的登录用户名",
                            value: "{username()}",
                            oninput: move |e| username.set(e.value()),
                        }
                    }

                    div { class: "flex flex-col gap-1.5",
                        span { class: "font-medium text-zinc-400", "密码" }
                        input {
                            r#type: "password",
                            class: "w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3.5 py-2.5 text-zinc-100 outline-none focus:border-purple-500 transition-colors",
                            placeholder: "输入密码",
                            value: "{password()}",
                            oninput: move |e| password.set(e.value()),
                        }
                    }

                    if is_register() {
                        div { class: "flex flex-col gap-1.5",
                            span { class: "font-medium text-zinc-400", "电子邮箱 (可选)" }
                            input {
                                class: "w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3.5 py-2.5 text-zinc-100 outline-none focus:border-purple-500 transition-colors",
                                placeholder: "user@example.com",
                                value: "{email()}",
                                oninput: move |e| email.set(e.value()),
                            }
                        }
                    }
                }

                // 提交按钮
                button {
                    class: "mt-1 flex items-center justify-center gap-2 w-full rounded-full bg-gradient-to-r from-purple-600 via-fuchsia-600 to-pink-600 py-3 text-xs font-bold text-white shadow-xl shadow-purple-600/30 hover:opacity-90 active:scale-[0.99] transition-all disabled:opacity-50",
                    disabled: loading(),
                    onclick: submit,
                    if loading() {
                        span { class: "h-3.5 w-3.5 animate-spin rounded-full border-2 border-white/20 border-t-white" }
                        span { "验证中…" }
                    } else if is_register() {
                        span { "立即注册并体验 ➜" }
                    } else {
                        span { "安全登录 ➜" }
                    }
                }

                div { class: "text-center text-[10px] text-zinc-500 pt-1",
                    "跨端通用安全鉴权中心 · 数据由 Argon2 与 HS256 JWT 加密保护"
                }
            }
        }
    }
}

/// 顶部栏用户态徽标: 未登录显示「登录/注册」，已登录显示头像和昵称菜单
#[component]
pub fn UserBadge(
    #[props(default)] on_open_login: EventHandler<()>,
    #[props(default)] user: Option<Option<UserDto>>,
    #[props(default)] on_logout: EventHandler<()>,
) -> Element {
    let mut local_user = use_signal(get_cached_user);
    let mut dropdown_open = use_signal(|| false);

    let current_user = match user {
        Some(u) => u,
        None => local_user(),
    };
    rsx! {
        div { class: "relative select-none",
            if let Some(user) = current_user {
                div {
                    class: "flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-900/90 px-2.5 py-1 text-xs cursor-pointer hover:border-purple-500/40 transition-colors",
                    onclick: move |_| dropdown_open.set(!dropdown_open()),
                    div { class: "flex h-5 w-5 items-center justify-center rounded-full bg-purple-600 text-[10px] font-bold text-white",
                        "{user.username.chars().next().unwrap_or('U')}"
                    }
                    span { class: "font-semibold text-zinc-200 max-w-[80px] truncate", "{user.display_name}" }
                    span { class: "text-[9px] text-zinc-500", "⌵" }
                }

                if dropdown_open() {
                    div {
                        class: "absolute right-0 top-full mt-2 z-50 w-44 rounded-2xl border border-zinc-800 bg-zinc-900/95 p-1.5 shadow-2xl backdrop-blur-2xl text-xs flex flex-col gap-1",
                        div { class: "px-2.5 py-2 border-b border-zinc-800/80 flex flex-col gap-0.5",
                            span { class: "font-bold text-white truncate", "{user.display_name}" }
                            span { class: "text-[10px] text-zinc-500 truncate", "@{user.username} · {user.role}" }
                        }
                        button {
                            class: "flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-zinc-300 hover:bg-zinc-800 hover:text-white transition-colors text-left",
                            onclick: move |_| dropdown_open.set(false),
                            span { "👤" }
                            span { "个人资料" }
                        }
                        button {
                            class: "flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-rose-400 hover:bg-rose-950/40 hover:text-rose-300 transition-colors text-left border-t border-zinc-800/80 mt-1",
                            onclick: move |_| {
                                clear_cached_session();
                                local_user.set(None);
                                on_logout.call(());
                                dropdown_open.set(false);
                            },
                            span { "🚪" }
                            span { "退出登录" }
                        }
                    }
                }
            } else {
                button {
                    class: "flex items-center gap-1.5 rounded-full bg-gradient-to-r from-purple-600 to-pink-600 px-3.5 py-1 text-xs font-bold text-white shadow-md shadow-purple-600/30 hover:scale-105 active:scale-95 transition-all",
                    onclick: move |_| on_open_login.call(()),
                    span { "🔑" }
                    span { "登录 / 注册" }
                }
            }
        }
    }
}
