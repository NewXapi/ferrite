//! SectionRail — 左侧 icon-only rail（Linear 风格，桌面常驻 / 移动端隐藏）。
//!
//! 契约：
//! - 三个 section 入口（总览/账户/管理），单色 SVG 图标 + hover 气泡 label
//! - 用户入口与额度显示统一由底部 StatusBar 承担，rail 不重复放置账号控件
//! - 移动端 `hidden md:flex`——移动端由顶部横向 section 条承担（对齐维护者
//!   「手机把侧边栏换成上下横向栏」的决定）

use dioxus::prelude::*;

use crate::icons::{IconChartBar, IconSettings, IconUser};
use crate::on_tab_wheel;

/// Rail 容器：窄列、边框分隔、纵向排布；移动端隐藏。
const RAIL_CLASS: &str = "hidden h-svh w-14 shrink-0 flex-col items-center border-r border-border bg-background py-3 md:flex";

/// 单个 rail 按钮态 class（激活=浅底深字，默认=灰字 hover 提亮）。
fn rail_button_class(active: bool) -> &'static str {
    if active {
        "group relative flex h-9 w-9 items-center justify-center rounded-lg bg-secondary text-foreground"
    } else {
        "group relative flex h-9 w-9 items-center justify-center rounded-lg text-muted-foreground hover:bg-card hover:text-foreground"
    }
}

/// hover 气泡：rail 右侧浮出 label。
const TOOLTIP_CLASS: &str = "pointer-events-none absolute left-full top-1/2 z-50 ml-2 -translate-y-1/2 whitespace-nowrap rounded-md border border-border bg-card px-2 py-1 text-xs text-foreground opacity-0 shadow-lg transition-opacity duration-150 group-hover:opacity-100 group-focus-visible:opacity-100";

/// 左侧 section rail。
///
/// - `active_index`：当前 section（0=总览 1=账户 2=管理）。
/// - `on_select`：点击 section 回调，参数为索引。
/// 用户入口由 `StatusBar` 统一承载，因此 rail 只负责 section 导航。
#[component]
pub fn SectionRail(
    /// 当前激活 section 索引。
    active_index: usize,
    /// section 点击回调（索引）。
    on_select: EventHandler<usize>,
) -> Element {
    let sections = [("总览", 0usize), ("账户", 1), ("管理", 2)];
    rsx! {
        aside {
            class: RAIL_CLASS,
            aria_label: "主导航",
            nav {
                class: "flex flex-col items-center gap-1",
                // 滚轮竖向滚动 → 循环切换 section(末项绕回首项)
                onwheel: move |e: WheelEvent| on_tab_wheel(e, sections.len(), active_index, |i| on_select.call(i)),
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
        }
    }
}
