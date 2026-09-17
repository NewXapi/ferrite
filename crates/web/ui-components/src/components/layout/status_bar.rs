//! StatusBar — 悬浮底部状态栏（双分块：左用户余额 + 右手柄系统状态）。
//!
//! 契约（维护者拍板）：不放具体业务数据呈现，只放假占位点/名称；真实数据后续
//! 通过 hover popover 注入。用户头像与登录入口已移到 SectionRail footer。

use dioxus::prelude::*;

/// 底部状态条目：占位名称 + 可选 hint（popover 接入前的静态说明）。
#[derive(Clone, PartialEq)]
pub struct StatusItem {
    /// 占位显示名（如「后端」「版本」）。
    pub label: String,
    /// 可选静态提示（不承载实时数据）。
    pub hint: Option<String>,
}

/// 左分块：用户余额占位。
fn user_balance_class() -> &'static str {
    "flex h-7 shrink-0 items-center gap-1.5 rounded-full border border-zinc-800 bg-zinc-900/80 px-2.5 text-[11px] text-zinc-300"
}

/// 右分块：手柄系统状态占位。
fn sys_state_class() -> &'static str {
    "flex h-7 shrink-0 items-center gap-1.5 rounded-full border border-zinc-800 bg-zinc-900/80 px-2.5 text-[11px] text-zinc-400"
}

/// 底部悬浮状态栏。
///
/// - `user_name`：登录用户名 —— None 时左分块占位「未登录」。
/// - `is_light` / `on_toggle_theme`：主题切换。
#[component]
pub fn StatusBar(
    /// 登录用户名；None 时左分块显示「未登录」。
    user_name: Option<String>,
    /// 当前是否浅色主题。
    is_light: bool,
    /// 切换主题回调。
    on_toggle_theme: EventHandler<()>,
    /// 状态条目（当前仅作为预留槽位，用不到时传空 vec）。
    #[props(default)]
    items: Vec<StatusItem>,
) -> Element {
    rsx! {
        div {
            class: "flex w-full max-w-3xl items-center justify-between gap-2 rounded-full border border-zinc-800/80 bg-zinc-900/90 px-3 py-0.5 shadow-lg shadow-black/20 backdrop-blur",
            // 左分块：用户余额(假占位 —— 真实余额来自账户页)
            div {
                class: user_balance_class(),
                span {
                    class: "h-1.5 w-1.5 shrink-0 rounded-full bg-emerald-500",
                    aria_hidden: "true",
                }
                match user_name {
                    Some(name) => rsx! {
                        span { class: "font-medium text-zinc-200", "{name}" }
                        span { class: "text-zinc-500", "· ¥——.--" }
                    },
                    None => rsx! {
                        span { class: "text-zinc-500", "未登录" }
                    },
                }
            }
            // 右分块：手柄系统状态(假占位)
            div {
                class: sys_state_class(),
                span {
                    class: "flex items-center gap-1",
                    span { class: "text-zinc-500", "CPU" }
                    span { class: "text-zinc-400", "——" }
                }
                span { class: "text-zinc-600", "·" }
                span {
                    class: "flex items-center gap-1",
                    span { class: "text-zinc-500", "MEM" }
                    span { class: "text-zinc-400", "——" }
                }
            }
            // 主题切换
            button {
                class: "shrink-0 rounded-full px-2 py-0.5 text-[11px] text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100",
                aria_label: "切换主题",
                onclick: move |_| on_toggle_theme.call(()),
                if is_light { "Dark" } else { "Light" }
            }
        }
    }
}
