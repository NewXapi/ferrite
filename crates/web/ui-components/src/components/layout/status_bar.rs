//! StatusBar — 底部悬浮状态栏（双分块：左下角用户头像+额度，右下角系统状态）。
//!
//! 契约：头像按钮点击展开下拉（账户资料/退出登录），占位数据后续通过 popover 注入。

use dioxus::prelude::*;

/// 底部状态条目：占位名称 + 可选 hint（popover 接入前的静态说明）。
#[derive(Clone, PartialEq)]
pub struct StatusItem {
    /// 占位显示名（如「后端」「版本」）。
    pub label: String,
    /// 可选静态提示（不承载实时数据）。
    pub hint: Option<String>,
}

/// 用户头像 chip class（圆形按钮 + 首字母）。
fn avatar_chip_class() -> &'static str {
    "flex h-6 w-6 shrink-0 items-center justify-center rounded-full border border-zinc-700 bg-zinc-800 text-[10px] font-semibold text-zinc-200 hover:border-zinc-600"
}

/// 用户下拉菜单 class。
const USER_MENU_CLASS: &str = "absolute bottom-full left-0 z-50 mb-2 w-36 rounded-lg border border-zinc-800 bg-zinc-900 p-1 shadow-xl";

/// 菜单项 class。
const MENU_ITEM_CLASS: &str =
    "block rounded-md px-2 py-1.5 text-xs text-zinc-200 hover:bg-zinc-800";

/// 系统状态 chip class。
fn sys_chip_class() -> &'static str {
    "flex h-6 shrink-0 items-center gap-1 rounded-full border border-zinc-800 bg-zinc-900/80 px-2 text-[10px] text-zinc-400"
}

/// 底部悬浮状态栏。
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
    let mut menu_open = use_signal(|| false);
    rsx! {
        div {
            class: "flex w-full max-w-3xl items-center justify-between gap-3 rounded-full border border-zinc-800/80 bg-zinc-900/90 px-2.5 py-0.5 shadow-lg shadow-black/20 backdrop-blur",
            // 左下角：用户头像 + 额度占位
            div {
                class: "flex items-center gap-2",
                match user_name {
                    Some(name) => rsx! {
                        div {
                            class: "relative",
                            button {
                                class: avatar_chip_class(),
                                "data-testid": "status-user-menu-button",
                                "aria-label": "用户菜单",
                                "aria-haspopup": "menu",
                                "aria-expanded": "{menu_open()}",
                                title: "{name}",
                                onclick: move |_| menu_open.toggle(),
                                "{name.chars().next().unwrap_or('?')}"
                            }
                            if menu_open() {
                                div {
                                    class: USER_MENU_CLASS,
                                    role: "menu",
                                    "aria-label": "用户菜单",
                                    a {
                                        class: MENU_ITEM_CLASS,
                                        "data-testid": "menu-account",
                                        role: "menuitem",
                                        href: "#account",
                                        onclick: move |_| menu_open.set(false),
                                        "账户资料"
                                    }
                                    div { class: "my-1 h-px bg-zinc-800" }
                                    button {
                                        class: "block w-full rounded-md px-2 py-1.5 text-left text-xs text-red-400 hover:bg-zinc-800 hover:text-red-300",
                                        "data-testid": "logout",
                                        role: "menuitem",
                                        onclick: move |_| {
                                            menu_open.set(false);
                                            on_logout.call(());
                                        },
                                        "退出登录"
                                    }
                                }
                            }
                        }
                        span { class: "text-[11px] text-zinc-500", "¥——.--" }
                    },
                    None => rsx! {
                        span { class: "text-[11px] text-zinc-500", "未登录" }
                    },
                }
            }
            // 右下角：系统状态占位 + 主题切换
            div {
                class: "flex items-center gap-1.5",
                div {
                    class: sys_chip_class(),
                    span { class: "text-zinc-500", "CPU" }
                    span { class: "text-zinc-400", "——" }
                }
                span { class: "text-zinc-600", "·" }
                div {
                    class: sys_chip_class(),
                    span { class: "text-zinc-500", "MEM" }
                    span { class: "text-zinc-400", "——" }
                }
                button {
                    class: "shrink-0 rounded-full px-1.5 py-0.5 text-[10px] text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100",
                    aria_label: "切换主题",
                    onclick: move |_| on_toggle_theme.call(()),
                    if is_light { "Dark" } else { "Light" }
                }
            }
        }
    }
}
