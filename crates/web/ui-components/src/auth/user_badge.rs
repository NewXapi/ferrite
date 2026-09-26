use contract::api::user::{UserDto, role_label};
use dioxus::prelude::*;

use crate::session::{clear_cached_session, get_cached_user};

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
                    class: "flex items-center gap-2 rounded-full border border-border bg-card/90 px-2.5 py-1 {crate::TYPE_DESC} cursor-pointer hover:border-purple-500/40 transition-colors",
                    onclick: move |_| dropdown_open.set(!dropdown_open()),
                    div { class: "flex h-5 w-5 items-center justify-center rounded-full bg-purple-600 {crate::TYPE_LABEL}",
                        "{user.username.chars().next().unwrap_or('U')}"
                    }
                    span { class: "font-semibold text-foreground max-w-[80px] truncate", "{user.display_name}" }
                    span { class: "{crate::TYPE_LABEL}", "⌵" }
                }

                if dropdown_open() {
                    div {
                        class: "absolute right-0 top-full mt-2 z-50 w-44 rounded-2xl border border-border bg-card/95 p-1.5 shadow-2xl backdrop-blur-2xl {crate::TYPE_DESC} flex flex-col gap-1",
                        div { class: "px-2.5 py-2 border-b border-border/80 flex flex-col gap-0.5",
                            span { class: "font-bold text-foreground truncate", "{user.display_name}" }
                            span { class: "{crate::TYPE_LABEL} truncate", "@{user.username} · {role_label(user.role)}" }
                        }
                        button {
                            class: "flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-foreground hover:bg-secondary hover:text-foreground transition-colors text-left",
                            onclick: move |_| dropdown_open.set(false),
                            span { "个人资料" }
                        }
                        button {
                            class: "flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-rose-400 hover:bg-rose-950/40 hover:text-rose-300 transition-colors text-left border-t border-border/80 mt-1",
                            onclick: move |_| {
                                clear_cached_session();
                                local_user.set(None);
                                on_logout.call(());
                                dropdown_open.set(false);
                            },
                            span { "退出登录" }
                        }
                    }
                }
            } else {
                button {
                    class: "flex items-center gap-1.5 rounded-full bg-gradient-to-r from-purple-600 to-pink-600 px-3.5 py-1 {crate::TYPE_DESC} shadow-md shadow-purple-600/30 hover:scale-105 active:scale-95 transition-all",
                    onclick: move |_| on_open_login.call(()),
                    span { "登录 / 注册" }
                }
            }
        }
    }
}
