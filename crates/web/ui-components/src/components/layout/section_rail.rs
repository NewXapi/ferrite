//! SectionRail — 左侧 icon-only rail（Linear 风格，桌面常驻 / 移动端隐藏）。
//!
//! 契约：
//! - 三个 section 入口（总览/账户/管理），单色 SVG 图标 + hover 气泡 label
//! - 底部 account footer：用户首字符圆点 + 名称（未登录显示「登录」链接）
//! - 移动端 `hidden md:flex`——移动端由顶部横向 section 条承担（对齐维护者
//!   「手机把侧边栏换成上下横向栏」的决定）

use dioxus::prelude::*;

use crate::icons::{IconChartBar, IconSettings, IconUser};

/// Rail 容器：窄列、边框分隔、纵向排布；移动端隐藏。
const RAIL_CLASS: &str = "hidden h-svh w-14 shrink-0 flex-col items-center border-r border-zinc-800 bg-zinc-950 py-3 md:flex";

/// 单个 rail 按钮态 class（激活=浅底深字，默认=灰字 hover 提亮）。
fn rail_button_class(active: bool) -> &'static str {
    if active {
        "group relative flex h-9 w-9 items-center justify-center rounded-lg bg-zinc-800 text-zinc-100"
    } else {
        "group relative flex h-9 w-9 items-center justify-center rounded-lg text-zinc-500 hover:bg-zinc-900 hover:text-zinc-200"
    }
}

/// hover 气泡：rail 右侧浮出 label。
const TOOLTIP_CLASS: &str = "pointer-events-none absolute left-full top-1/2 z-50 ml-2 -translate-y-1/2 whitespace-nowrap rounded-md border border-zinc-700 bg-zinc-900 px-2 py-1 text-xs text-zinc-200 opacity-0 shadow-lg transition-opacity duration-150 group-hover:opacity-100 group-focus-visible:opacity-100";

/// 左侧 section rail。
///
/// - `active_index`：当前 section（0=总览 1=账户 2=管理）。
/// - `on_select`：点击 section 回调，参数为索引。
/// - `user_name`：登录用户名；Some 时 footer 显示用户下拉，None 时 footer 留空。
/// - `on_logout`：退出登录回调（footer 下拉用）。
#[component]
pub fn SectionRail(
    /// 当前激活 section 索引。
    active_index: usize,
    /// section 点击回调（索引）。
    on_select: EventHandler<usize>,
    /// 登录用户名；None 时 footer 留空。
    user_name: Option<String>,
    /// 退出登录回调。
    on_logout: EventHandler<()>,
) -> Element {
    let sections = [("总览", 0usize), ("账户", 1), ("管理", 2)];
    let mut menu_open = use_signal(|| false);
    rsx! {
        aside {
            class: RAIL_CLASS,
            aria_label: "主导航",
            nav {
                class: "flex flex-col items-center gap-1",
                for (label, idx) in sections {
                    button {
                        key: "{idx}",
                        class: rail_button_class(active_index == idx),
                        aria_label: "{label}",
                        onclick: move |_| on_select.call(idx),
                        if idx == 0 {
                            IconChartBar { size: 18 }
                        } else if idx == 1 {
                            IconUser { size: 18 }
                        } else {
                            IconSettings { size: 18 }
                        }
                        span { class: TOOLTIP_CLASS, "{label}" }
                    }
                }
            }
            // account footer：容器保留；登录态=用户下拉(账户资料/退出登录)，未登录=留空
            div {
                class: "mt-auto flex flex-col items-center gap-1 border-t border-zinc-800/80 pt-2",
                match user_name {
                    Some(name) => rsx! {
                        div {
                            class: "relative",
                            button {
                                class: "flex h-8 w-8 items-center justify-center rounded-full border border-zinc-700 bg-zinc-800 text-xs font-semibold text-zinc-200 hover:border-zinc-600",
                                "data-testid": "user-menu-button",
                                "aria-label": "用户菜单",
                                "aria-haspopup": "menu",
                                "aria-expanded": "{menu_open()}",
                                title: "{name}",
                                onclick: move |_| menu_open.toggle(),
                                "{name.chars().next().unwrap_or('?')}"
                            }
                            if menu_open() {
                                div {
                                    class: "absolute bottom-full left-1/2 z-50 mb-2 w-36 -translate-x-1/2 rounded-lg border border-zinc-800 bg-zinc-900 p-1 shadow-xl",
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
                    // 未登录：footer 容器保留、内容清空（登录入口由调用方决定）
                    None => rsx! {},
                }
            }
        }
    }
}
