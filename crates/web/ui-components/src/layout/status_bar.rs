//! StatusBar — 底部细状态条（无背景胶囊，整条一个字高）。
//!
//! 契约（维护者拍板）：不做任何背景/边框/阴影包装，左下角=用户头像+额度占位，
//! 右下角=系统状态纯数字占位（CPU·MEM 顺序，含义走 title 悬停提示）；
//! 真实数据后续通过 hover popover 注入（组件留 `StatusItem.hint` 槽位）。
//! 用户区委托 [`avatar_menu::AvatarMenu`]（rust-ui Avatar 触发 + 向上弹出菜单：
//! 行 1 头像+用户名、行 2 余额、行 3+ 页签、末尾退出登录）。

use dioxus::prelude::*;

use crate::layout::avatar_menu::AvatarMenu;

/// 底部状态条目：占位名称 + 可选 hint（popover 接入前的静态说明）。
#[derive(Clone, PartialEq)]
pub struct StatusItem {
    /// 占位显示名（如「后端」「版本」）。
    pub label: String,
    /// 可选静态提示（不承载实时数据）。
    pub hint: Option<String>,
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
    /// 用户菜单的页签名列表（当前 section 的页签；AvatarMenu 行 3+）。
    #[props(default)]
    menu_tabs: Vec<String>,
    /// 当前激活页签下标（菜单内高亮）。
    #[props(default)]
    active_tab: u8,
    /// 点击菜单内页签行（与 TopNavBar 同一信号语义，调用方写回 dash_tab）。
    #[props(default)]
    on_tab_select: EventHandler<usize>,
) -> Element {
    rsx! {
        div {
            class: "flex w-full items-center justify-between py-0.5 text-[11px] text-zinc-500",
            // 左下角：用户头像 + 额度占位
            div {
                class: "flex items-center gap-1.5",
                match user_name {
                    Some(name) => rsx! {
                        AvatarMenu {
                            user_name: name,
                            amount: "¥——.--".to_string(),
                            menu_tabs: menu_tabs,
                            active_tab: active_tab as usize,
                            on_tab_select: on_tab_select,
                            on_logout: on_logout,
                        }
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
