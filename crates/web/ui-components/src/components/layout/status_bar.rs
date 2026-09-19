//! StatusBar — 底部细状态条（无背景胶囊，整条一个字高）。
//!
//! 契约（维护者拍板）：不做任何背景/边框/阴影包装，左下角=用户头像+额度占位，
//! 右下角=系统状态纯数字占位（CPU·MEM 顺序，含义走 title 悬停提示）；
//! 真实数据后续通过 hover popover 注入（组件留 `StatusItem.hint` 槽位）。
//! 用户下拉复用 crate 的 DropdownMenu（含外部点击/Escape 关闭，选中即关对齐 Radix 默认）。

use dioxus::prelude::*;

use crate::components::dropdown_menu::{DropdownMenu, DropdownMenuItem, DropdownMenuSeparator};

/// 底部状态条目：占位名称 + 可选 hint（popover 接入前的静态说明）。
#[derive(Clone, PartialEq)]
pub struct StatusItem {
    /// 占位显示名（如「后端」「版本」）。
    pub label: String,
    /// 可选静态提示（不承载实时数据）。
    pub hint: Option<String>,
}

/// 用户头像 chip class（16px 圆点 + 首字母，对齐单字行高）。
fn avatar_chip_class() -> &'static str {
    "flex h-4 w-4 shrink-0 items-center justify-center rounded-full bg-zinc-700 text-[9px] font-semibold text-zinc-200 hover:bg-zinc-600"
}

/// 底部细状态条（无背景，单行文字高度）。
///
/// - `user_name`：登录用户名；Some 时显示头像+下拉菜单，None 时占位「未登录」。
/// - `is_light` / `on_toggle_theme`：主题切换。
/// - `on_logout`：退出登录回调（头像下拉用）。
#[component]
pub fn StatusBar(
    /// 登录用户名；None 时显示「未登录」占位。
    user_name: Option<String>,
    /// 当前是否浅色主题。
    is_light: bool,
    /// 切换主题回调。
    on_toggle_theme: EventHandler<()>,
    /// 退出登录回调。
    on_logout: EventHandler<()>,
    /// 状态条目（当前仅作为预留槽位，用不到时传空 vec）。
    #[props(default)]
    items: Vec<StatusItem>,
) -> Element {
    // 外部关闭请求信号（「面板应关闭？」默认 false）：点选中项置 true，DropdownMenu 收关
    let close_request = use_signal(|| false);
    let mut close_signal = close_request;
    rsx! {
        div {
            class: "flex w-full items-center justify-between py-0.5 text-[11px] text-zinc-500",
            // 左下角：用户头像 + 额度占位
            div {
                class: "flex items-center gap-1.5",
                match user_name {
                    Some(name) => rsx! {
                        div {
                            class: "relative",
                            DropdownMenu {
                                trigger: rsx! {
                                    button {
                                        class: avatar_chip_class(),
                                        "data-testid": "status-user-menu-button",
                                        "aria-label": "用户菜单",
                                        title: "{name}",
                                        "{name.chars().next().unwrap_or('?')}"
                                    }
                                },
                                content: rsx! {
                                    DropdownMenuItem {
                                        onclick: move |_| close_signal.set(true),
                                        "data-testid": "menu-account",
                                        // 锚点在 item 内：点击冒泡到 item（组件统一收关）后跳转
                                        a {
                                            class: "block",
                                            href: "#account",
                                            "账户资料"
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
                                content_class: Some("bottom-full left-0 mb-2 w-36".into()),
                                close_signal: Some(close_request),
                            }
                        }
                        span { class: "text-zinc-500", "¥——.--" }
                    },
                    None => rsx! {
                        span { class: "text-zinc-500", "未登录" }
                    },
                }
            }
            // 右下角：系统状态纯数字占位（CPU · MEM，含义走 title）+ 主题切换
            div {
                class: "flex items-center gap-2",
                span {
                    class: "text-zinc-500",
                    title: "CPU",
                    "data-testid": "status-cpu",
                    "12%"
                }
                span { class: "text-zinc-600", "·" }
                span {
                    class: "text-zinc-500",
                    title: "内存",
                    "data-testid": "status-mem",
                    "34%"
                }
                button {
                    class: "shrink-0 rounded px-1 text-[11px] text-zinc-500 transition-colors hover:text-zinc-300",
                    aria_label: "切换主题",
                    onclick: move |_| on_toggle_theme.call(()),
                    if is_light { "Dark" } else { "Light" }
                }
            }
        }
    }
}
