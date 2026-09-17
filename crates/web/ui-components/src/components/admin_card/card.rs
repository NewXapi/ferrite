use dioxus::prelude::*;

use super::dot_tab::DotTabBar;

/// 渲染管理页实体摘要的共享卡片外壳。
///
/// `title` 和可选的 `subtitle` 用作卡片标题；`tabs` 是只读内容页签的标签，
/// `active_tab` 指明当前圆点，`on_tab_change` 接收用户选择的索引。`children`
/// 是当前页签对应的内容。`show_edit` 为真时显示编辑入口，并由 `on_edit` 接收
/// 点击；该入口仅供将来接入 Popover，当前不实现编辑界面。`testid` 可为整个
/// 卡片指定测试标识。
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
    /// 将来可接 Popover 的编辑入口回调；未传时仍可由 `show_edit` 控制入口显示。
    #[props(default)]
    on_edit: EventHandler<()>,
    /// 是否渲染编辑入口（未来可接 Popover）。
    #[props(default)]
    show_edit: bool,
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
                    // 编辑入口（未来可接 Popover）。
                    if show_edit {
                        button {
                            class: "rounded p-1 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                            onclick: move |_| on_edit.call(()),
                            "aria-label": "编辑入口（未来可接 Popover）",
                            "data-testid": "admin-card-edit",
                            type: "button",
                            svg {
                                class: "h-3.5 w-3.5",
                                view_box: "0 0 24 24",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "2",
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                path { d: "M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" }
                                path { d: "M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" }
                            }
                        }
                    }
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
