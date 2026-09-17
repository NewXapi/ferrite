use dioxus::prelude::*;

use super::dot_tab::DotTabBar;

/// 渲染管理页实体摘要的共享卡片外壳。
///
/// `title` 和可选的 `subtitle` 用作卡片标题；`tabs` 是只读内容页签的标签，
/// `active_tab` 指明当前圆点，`on_tab_change` 接收用户选择的索引。`children`
/// 是当前页签对应的内容。`testid` 可为整个卡片指定测试标识。
///
/// 当 `tabs` 为空或 `active_tab` 超出 `tabs` 范围时不会产生错误：前者不渲染
/// 圆点，后者不激活任何圆点。
///
/// 例如，实体卡可传入两个摘要页签，并在回调中切换其本地内容状态。
#[component]
pub fn AdminCard(
    title: String,
    subtitle: Option<String>,
    tabs: Vec<&'static str>,
    active_tab: usize,
    on_tab_change: EventHandler<usize>,
    children: Element,
    /// 整张卡片的可选测试标识；未传时渲染空值。
    #[props(default)]
    testid: Option<String>,
) -> Element {
    rsx! {
        div {
            class: "group flex flex-col rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            role: "region",
            "aria-label": "{title}",
            "data-testid": testid.unwrap_or_default(),

            // Header: title + dot tabs at top-right.
            div { class: "flex items-start justify-between gap-3",
                div { class: "min-w-0 flex-1",
                    h3 { class: "truncate text-sm font-medium text-zinc-100", "{title}" }
                    if let Some(sub) = subtitle {
                        p { class: "mt-0.5 truncate text-[11px] text-zinc-400", "{sub}" }
                    }
                }
                div { class: "flex items-center gap-2 pt-0.5",
                    DotTabBar {
                        tabs: tabs.clone(),
                        active: active_tab,
                        on_change: on_tab_change,
                    }
                }
            }

            // Tab content (no bottom button row).
            div { class: "mt-3", {children} }
        }
    }
}
