//! TopNavBar — 顶部 page-tab 行（沿用原 TabItem 视觉契约：文字 + 激活项底部
//! 0.5px 白色下划线），置于顶栏左上角，不再使用胶囊容器。
//!
//! 组件只渲染 tab 集；section 切换在桌面由 SectionRail、移动端由调用方在
//! 顶栏 slot 里自行加横向 section 条。

use dioxus::prelude::*;

use crate::on_tab_wheel;

/// 单个 tab 态 class（激活=白字+底线下划线，默认=灰字 hover 提亮）。h-6 比 h-7 矮一档。
fn tab_class(active: bool) -> &'static str {
    if active {
        "relative flex h-6 shrink-0 items-center px-2 text-sm font-medium text-zinc-100"
    } else {
        "relative flex h-6 shrink-0 items-center px-2 text-sm font-medium text-zinc-500 transition-colors hover:text-zinc-300"
    }
}

/// 激活项底部下划线（对齐原 TabItem：inset-x-2 高 0.5 白条）。
const UNDERLINE_CLASS: &str =
    "pointer-events-none absolute inset-x-2 bottom-0 h-0.5 rounded-full bg-zinc-100";

/// 顶部 page-tab 行（左上角，无边框容器）。
///
/// - `tabs`：当前 section 的页面 tab 文案。
/// - `active`：激活 tab 索引。
/// - `on_select`：点击回调（索引）。
#[component]
pub fn TopNavBar(
    /// 当前 section 的页面 tab 文案。
    tabs: Vec<String>,
    /// 激活 tab 索引。
    active: usize,
    /// 点击回调（索引）。
    on_select: EventHandler<usize>,
) -> Element {
    rsx! {
        nav {
            class: "flex max-w-full items-center gap-1 overflow-x-auto whitespace-nowrap",
            aria_label: "页面导航",
            // 滚轮竖向滚动 → 循环切换页内 tab(末项绕回首项)
            onwheel: move |e: WheelEvent| on_tab_wheel(e, tabs.len(), active, |i| on_select.call(i)),
            for (i, label) in tabs.iter().enumerate() {
                button {
                    key: "{i}",
                    class: tab_class(active == i),
                    aria_label: "{label}",
                    onclick: move |_| on_select.call(i),
                    "{label}"
                    if active == i {
                        span { class: UNDERLINE_CLASS }
                    }
                }
            }
        }
    }
}
