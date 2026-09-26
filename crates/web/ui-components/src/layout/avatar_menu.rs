//! AvatarMenu — 状态栏左下角用户菜单（rust-ui Avatar 触发 + 向上弹出菜单）。
//!
//! 批注 #account(24a2abab) 拍板的行结构（一行一个内容）：
//! 行 1 头像 Avatar + 用户名；行 2 余额（左对齐）；行 3+ 当前页签（一行一项，
//! 激活项高亮）；末尾保留既有「退出登录」。触发器即 rust-ui Avatar（图片可选，
//! 无图回落首字符）；菜单容器复用 crate 的 DropdownMenu（`content_class`
//! `bottom-full` 向上弹，「选中即关」语义与 StatusBar 原实现一致）。

use dioxus::prelude::*;

use crate::components::rui_avatar::{Avatar, AvatarFallback, AvatarImage};
use crate::dropdown_menu::{DropdownMenu, DropdownMenuItem, DropdownMenuSeparator};

/// 触发器头像：rust-ui 默认 `size-8`，状态栏单字行高下压到 `size-7`。
const TRIGGER_AVATAR_CLASS: &str = "size-7";

/// 激活页签行的高亮 class（shadcn Item 的 focus 态配色复用为选中态）。
const ACTIVE_TAB_CLASS: &str = "bg-accent text-accent-foreground";

/// 状态栏左下角用户菜单（向上弹出）。
///
/// - `user_name`：登录用户名（菜单行 1 与头像 fallback 共用）。
/// - `avatar_src`：头像图片；`None` 渲染首字符 fallback。
/// - `amount`：行 2 的余额文字（如 "¥——.--"，占位或真实值由调用方决定）。
/// - `menu_tabs`：当前页面的页签名列表（行 3+，顺序即展示顺序）。
/// - `active_tab`：当前激活页签下标（菜单内高亮行）。
/// - `on_tab_select`：点击页签行（参数为下标；与 TopNavBar 同一信号语义，
///   由调用方写回 `dash_tab`，不改 hash，与现有页签行为一致）。
/// - `on_logout`：退出登录。
#[component]
pub fn AvatarMenu(
    user_name: String,
    #[props(default)] avatar_src: Option<String>,
    amount: String,
    menu_tabs: Vec<String>,
    #[props(default)] active_tab: usize,
    on_tab_select: EventHandler<usize>,
    on_logout: EventHandler<()>,
) -> Element {
    // 外部关闭请求信号：点中项置 true，DropdownMenu 收关（与 StatusBar 原实现同语义）
    let close_request = use_signal(|| false);
    let mut close_signal = close_request;
    let initial = user_name.chars().next().unwrap_or('?');

    rsx! {
        div { class: "relative",
            DropdownMenu {
                trigger: rsx! {
                    button {
                        class: "rounded-full outline-none focus-visible:ring-2 focus-visible:ring-ring/50",
                        "data-testid": "status-user-menu-button",
                        "aria-label": "用户菜单",
                        title: "{user_name}",
                        Avatar {
                            class: TRIGGER_AVATAR_CLASS,
                            if let Some(src) = avatar_src.clone() {
                                AvatarImage { src: "{src}", alt: "{user_name}" }
                            } else {
                                AvatarFallback { "{initial}" }
                            }
                        }
                    }
                },
                content: rsx! {
                    // 行 1：头像 + 用户名
                    div { class: "flex items-center gap-2 px-2 py-1.5",
                        Avatar {
                            if let Some(src) = avatar_src {
                                AvatarImage { src: "{src}", alt: "{user_name}" }
                            } else {
                                AvatarFallback { "{initial}" }
                            }
                        }
                        span { class: "truncate text-sm font-medium text-zinc-100", "{user_name}" }
                    }
                    // 行 2：余额（左对齐）
                    div { class: "px-2 pb-1.5 text-xs text-zinc-500", "{amount}" }
                    DropdownMenuSeparator {}
                    // 行 3+：当前页面页签，一行一项
                    for (i, label) in menu_tabs.iter().enumerate() {
                        {
                            // 首行沿用旧「账户资料」项的 testid（e2e 兼容），其余按序号
                            let testid = if i == 0 { "menu-account".to_string() } else { format!("menu-tab-{i}") };
                            rsx! {
                                DropdownMenuItem {
                                    key: "{label}",
                                    class: if i == active_tab { ACTIVE_TAB_CLASS } else { "" },
                                    onclick: move |_| {
                                        on_tab_select.call(i);
                                        close_signal.set(true);
                                    },
                                    "data-testid": "{testid}",
                                    "{label}"
                                }
                            }
                        }
                    }
                    DropdownMenuSeparator {}
                    DropdownMenuItem {
                        onclick: move |_| {
                            close_signal.set(true);
                            on_logout.call(());
                        },
                        "data-testid": "logout",
                        "data-variant": "destructive",
                        "退出登录"
                    }
                },
                content_class: Some("bottom-full left-0 mb-2 w-44".into()),
                close_signal: Some(close_request),
            }
        }
    }
}
