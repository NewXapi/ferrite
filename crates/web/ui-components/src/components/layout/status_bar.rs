//! StatusBar — 悬浮底部状态栏（占位命名 + 主题切换 + 账户入口）。
//!
//! 契约（维护者拍板）：不放具体数据呈现，只放假占位点/名称；真实数据后续
//! 通过 hover popover 注入（组件留 `StatusItem.hint` 槽位）。

use dioxus::prelude::*;

/// 底部状态条目：占位名称 + 可选 hint（popover 接入前的静态说明）。
#[derive(Clone, PartialEq)]
pub struct StatusItem {
    /// 占位显示名（如「后端」「版本」）。
    pub label: String,
    /// 可选静态提示（不承载实时数据）。
    pub hint: Option<String>,
}

/// 状态项 chip class：小圆点 + 名称的占位样式。
const ITEM_CLASS: &str = "inline-flex items-center gap-1.5 rounded-full border border-zinc-800 bg-zinc-900/80 px-2 py-0.5 text-[11px] text-zinc-400";

/// 右侧操作按钮 class。
const ACTION_CLASS: &str = "rounded-full px-2.5 py-0.5 text-[11px] text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100";

/// 底部悬浮状态栏。
///
/// - `items`：左侧占位条目。
/// - `is_light` / `on_toggle_theme`：主题切换（继承原顶栏按钮语义）。
/// - `user_name`：Some=显示账户 chip + 下拉（退出登录）；None=显示「登录」链接。
/// - `on_logout`：退出登录回调。
#[component]
pub fn StatusBar(
    /// 左侧占位条目。
    items: Vec<StatusItem>,
    /// 当前是否浅色主题。
    is_light: bool,
    /// 切换主题回调。
    on_toggle_theme: EventHandler<()>,
    /// 登录用户名；None 显示登录链接。
    user_name: Option<String>,
    /// 退出登录回调。
    on_logout: EventHandler<()>,
) -> Element {
    let mut menu_open = use_signal(|| false);
    rsx! {
        div {
            class: "flex w-full max-w-3xl items-center justify-between gap-2 rounded-full border border-zinc-800/80 bg-zinc-900/90 px-3 py-1 shadow-lg shadow-black/20 backdrop-blur",
            // 左侧：占位条目（假数据位）
            div {
                class: "flex min-w-0 items-center gap-1.5 overflow-x-auto",
                for item in &items {
                    span {
                        key: "{item.label}",
                        class: ITEM_CLASS,
                        span {
                            class: "h-1.5 w-1.5 shrink-0 rounded-full bg-zinc-600",
                            aria_hidden: "true",
                        }
                        span { class: "shrink-0", "{item.label}" }
                        if let Some(hint) = &item.hint {
                            span { class: "shrink-0 text-zinc-600", "· {hint}" }
                        }
                    }
                }
            }
            // 右侧：主题 + 账户
            div {
                class: "flex shrink-0 items-center gap-1",
                button {
                    class: ACTION_CLASS,
                    aria_label: "切换主题",
                    onclick: move |_| on_toggle_theme.call(()),
                    if is_light { "Dark" } else { "Light" }
                }
                match user_name {
                    Some(name) => rsx! {
                        div {
                            class: "relative",
                            button {
                                class: "flex items-center gap-1.5 rounded-full border border-zinc-700 bg-zinc-800 px-2 py-0.5 text-[11px] font-medium text-zinc-200 hover:border-zinc-600",
                                "data-testid": "user-menu-button",
                                "aria-haspopup": "menu",
                                "aria-expanded": "{menu_open()}",
                                onclick: move |_| menu_open.toggle(),
                                span {
                                    class: "flex h-4 w-4 items-center justify-center rounded-full bg-zinc-700 text-[9px]",
                                    "{name.chars().next().unwrap_or('?')}"
                                }
                                "{name}"
                            }
                            if menu_open() {
                                div {
                                    class: "absolute bottom-full right-0 z-50 mb-2 w-36 rounded-lg border border-zinc-800 bg-zinc-900 p-1 shadow-xl",
                                    role: "menu",
                                    "aria-label": "用户菜单",
                                    a {
                                        class: "block rounded-md px-2 py-1.5 text-xs text-zinc-200 hover:bg-zinc-800",
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
                    },
                    None => rsx! {
                        a {
                            class: "rounded-full bg-zinc-100 px-2.5 py-0.5 text-[11px] font-medium text-zinc-900 hover:bg-zinc-300",
                            href: "#signup",
                            "登录"
                        }
                    },
                }
            }
        }
    }
}
