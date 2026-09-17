//! TopNavBar — 悬浮顶部 page-tab 胶囊（继承原 ConsolePanel header tab 语义）。
//!
//! Linear 风格：顶部居中悬浮 pill，横向滚动兜底（Manage 9 个 tab 在窄屏溢出）。
//! 组件只渲染 tab 集；section 切换在桌面由 SectionRail、移动端由调用方在
//! 顶栏 slot 里自行加横向 section 条。

use dioxus::prelude::*;

/// 胶囊容器：居中、悬浮、毛玻璃。
const PILL_CLASS: &str = "flex max-w-full items-center gap-1 overflow-x-auto whitespace-nowrap rounded-full border border-zinc-800/80 bg-zinc-900/90 px-2 py-1 shadow-lg shadow-black/20 backdrop-blur";

/// 单个 tab 态 class（激活=浅底深字，默认=灰字 hover 提亮）。
fn tab_class(active: bool) -> &'static str {
    if active {
        "relative flex h-7 shrink-0 items-center rounded-full bg-zinc-100 px-3 text-sm font-semibold text-zinc-900"
    } else {
        "relative flex h-7 shrink-0 items-center rounded-full px-3 text-sm font-medium text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100"
    }
}

/// 顶部悬浮 page-tab 胶囊。
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
            class: PILL_CLASS,
            aria_label: "页面导航",
            for (i, label) in tabs.iter().enumerate() {
                button {
                    key: "{i}",
                    class: tab_class(active == i),
                    aria_label: "{label}",
                    onclick: move |_| on_select.call(i),
                    "{label}"
                }
            }
        }
    }
}
