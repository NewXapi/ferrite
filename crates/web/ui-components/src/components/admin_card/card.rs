use dioxus::prelude::*;

use super::dot_tab::DotTabBar;

/// 渲染管理页实体摘要的共享卡片外壳。
///
/// `title` 和可选的 `subtitle` 用作卡片标题；`tabs` 是只读内容页签的标签，
/// `active_tab` 指明当前圆点，`on_tab_change` 接收用户选择的索引。
///
/// `panel_0` 到 `panel_3` 是四个页签各自的内容。四个 panel 始终同格渲染
/// （grid 叠加在 `col-start-1 row-start-1`），非激活的加 `invisible`
/// （`visibility:hidden`，仍占布局高度）与 `pointer-events-none`，因此容器
/// 高度由最高的 panel 决定，切换页签时卡片高度不跳动。`testid` 可为整张
/// 卡片指定测试标识。
///
/// 当 `tabs` 为空时不渲染圆点；`active_tab` 超出 `tabs` 范围时不激活任何圆点。
///
/// 例如，实体卡可传入四个摘要页签内容，并在回调中切换其本地页签状态。
#[component]
pub fn AdminCard(
    title: String,
    subtitle: Option<String>,
    tabs: Vec<&'static str>,
    active_tab: usize,
    on_tab_change: EventHandler<usize>,
    /// 页签 0（基本信息类页签）的内容。
    panel_0: Element,
    /// 页签 1 的内容。
    panel_1: Element,
    /// 页签 2 的内容。
    panel_2: Element,
    /// 页签 3（系统类页签）的内容。
    panel_3: Element,
    /// 整张卡片的可选测试标识；未传时渲染空值。
    #[props(default)]
    testid: Option<String>,
) -> Element {
    // 非激活页签：invisible（占布局高度）+ pointer-events-none（不可交互）。
    // 类名必须完整字面量出现在源码里，Tailwind 才会生成对应 CSS（动态拼串不会被扫描）。
    const PANEL_ON: &str = "col-start-1 row-start-1";
    const PANEL_OFF: &str = "col-start-1 row-start-1 invisible pointer-events-none";
    let c0 = if active_tab == 0 { PANEL_ON } else { PANEL_OFF };
    let c1 = if active_tab == 1 { PANEL_ON } else { PANEL_OFF };
    let c2 = if active_tab == 2 { PANEL_ON } else { PANEL_OFF };
    let c3 = if active_tab == 3 { PANEL_ON } else { PANEL_OFF };
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

            // Tab content: 四个 panel 同格叠加，容器高度取最高者，切页签不跳动。
            div { class: "mt-3 grid grid-cols-1",
                div { class: "{c0}", {panel_0} }
                div { class: "{c1}", {panel_1} }
                div { class: "{c2}", {panel_2} }
                div { class: "{c3}", {panel_3} }
            }
        }
    }
}
