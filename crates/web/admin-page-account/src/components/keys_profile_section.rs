//! keys 面板个人资料区段组件：明信片排版（rust-ui Card + Avatar 重构，批注 52a55348）。
//!
//! 纯展示组件——数据由页面层经 props 传入,本组件不发请求、不改状态。

use dioxus::prelude::*;

use contract::api::user::UserDto;

use ui::components::rui_avatar::{Avatar, AvatarFallback};
use ui::components::rui_badge::{Badge, BadgeSize, BadgeVariant};
use ui::components::rui_button::{Button, ButtonSize, ButtonVariant};
use ui::components::rui_card::Card;
use ui::components::rui_separator::Separator;

use crate::usage_support::short_key;

/// 区段标题文案。
const SEC_PROFILE: &str = "个人资料";

/// 【是什么】个人资料明信片组件，头像 + 用户名 + 角色与无标签资料值。
///
/// 【做什么】负责渲染「个人资料」区明信片：Avatar + 用户名 + 角色 Badge，分隔线下直接陈列邮箱 / 用户ID (可复制) / 注册时间的值本身——不写「邮箱」「用户ID」这类前缀字眼，显示名与用户名重复故不渲染（批注 52a55348）。不负责数据获取，仅做纯展示与加载/错误态分发。
///
/// 【交互逻辑】点「复制」：写用户ID 明文到剪贴板，成功后按钮短暂切换「已复制」。其余纯展示。
///
/// 【样式】rust-ui Card 承载（p-6，hover 边框变亮）；Avatar 默认档 + 首字符 fallback；角色 Badge Secondary Sm；资料值 text-sm zinc-300，用户ID 等宽字体。
///
/// 【子组件组成】rui Card + Avatar/AvatarFallback + Badge + Separator + Button (复制)
///
/// 【数据流】通过 props 接收：user (Option<UserDto>)、self_err (String)、pending (String)。无输出。
#[component]
pub fn KeysProfileSection(user: Option<UserDto>, self_err: String, pending: String) -> Element {
    let mut copied = use_signal(|| false);

    rsx! {
        section {
            id: "keys-sec-profile",
            class: "scroll-mt-8 space-y-3",
            h2 { class: "text-lg font-medium text-zinc-100", "{SEC_PROFILE}" }
            if let Some(user) = user {
                Card { class: "gap-4 bg-zinc-900/60 px-6 py-5 transition-colors hover:border-zinc-600",
                    div { class: "flex items-center gap-3",
                        Avatar {
                            AvatarFallback { "{user.username.chars().next().unwrap_or('?')}" }
                        }
                        div { class: "min-w-0",
                            div { class: "truncate text-base font-semibold text-zinc-100", "{user.username}" }
                            Badge {
                                variant: BadgeVariant::Secondary,
                                size: BadgeSize::Sm,
                                class: "mt-0.5 rounded-full font-normal text-zinc-400",
                                "{contract::api::user::role_label(user.role)}"
                            }
                        }
                    }
                    Separator { class: "bg-zinc-800" }
                    div { class: "flex flex-wrap items-center gap-x-10 gap-y-3 text-sm text-zinc-300",
                        if user.email.is_empty() { span { "—" } } else { span { "{user.email}" } }
                        span { class: "inline-flex items-center gap-2",
                            span { class: "font-mono", "{short_key(&user.key)}" }
                            Button {
                                variant: ButtonVariant::Ghost,
                                size: ButtonSize::Sm,
                                class: "h-6 px-2 text-xs text-zinc-400",
                                aria_label: "复制用户ID",
                                onclick: move |_| {
                                    // fire-and-forget 写入剪贴板, 提交即视为发起成功
                                    let ok = ui::copy_text_to_clipboard(&user.key);
                                    copied.set(ok);
                                    let mut c = copied;
                                    spawn(async move {
                                        gloo_timers::future::TimeoutFuture::new(1500).await;
                                        c.set(false);
                                    });
                                },
                                if copied() { "已复制" } else { "复制" }
                            }
                        }
                        span { class: "font-mono text-zinc-400", "{user.created_at.chars().take(10).collect::<String>()}" }
                    }
                }
            } else if !self_err.is_empty() {
                div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-6 text-sm text-amber-400",
                    "无法加载用户信息 (未登录或请求失败): {self_err}"
                }
            } else {
                div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-6 text-sm text-zinc-500", "{pending}" }
            }
        }
    }
}
