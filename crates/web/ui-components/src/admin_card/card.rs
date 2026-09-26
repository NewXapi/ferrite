use dioxus::prelude::*;

use super::dot_tab::DotTabBar;
use super::shell::CARD_SHELL_CLASS;

/// Shortens a key by Unicode scalar value without splitting UTF-8 characters.
///
/// Keeps at most `EDGE_CHARS * 2` (head…tail) characters so entity keys stay
/// readable without wrapping; shared by all admin entity cards in this module.
pub(crate) fn short_key(key: &str) -> String {
    const EDGE_CHARS: usize = 4;

    let char_count = key.chars().count();
    if char_count <= EDGE_CHARS * 2 {
        return key.to_string();
    }

    let head: String = key.chars().take(EDGE_CHARS).collect();
    let tail: String = key.chars().skip(char_count - EDGE_CHARS).collect();
    format!("{head}…{tail}")
}

/// 渲染管理页实体摘要的共享卡片外壳。
///
/// `title` 和可选的 `subtitle` 用作卡片标题；`tabs` 是只读内容页签的标签，
/// `active_tab` 指明当前圆点，`on_tab_change` 接收用户选择的索引。
///
/// `panel_0` 到 `panel_2` 是前三个页签的内容，`panel_3` 可选（不传即该槽位
/// 不渲染，供最多 3 个页签的实体卡使用）。各 panel 始终同格渲染
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
    /// 页签 3（系统类页签）的内容；不传（或显式 None）时该槽位不渲染，
    /// 供 ≤3 页签的实体卡使用（决策记录 §2.3：卡牌内 tab 最多 3 个）。
    #[props(default)]
    panel_3: Option<Element>,
    /// 整张卡片的可选测试标识；未传时渲染空值。
    #[props(default)]
    testid: Option<String>,
    /// 标题栏右侧、圆点页签之前的可选插槽（如实体卡的序号 badge）。
    #[props(default)]
    header_action: Option<Element>,
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
            // 卡片外壳 class 与各实体卡共用 shell::CARD_SHELL_CLASS（含 justify-between，
            // 与旧卡几何一致）；此前本组件自带一份少 justify-between 的拷贝。
            class: "{CARD_SHELL_CLASS}",
            role: "region",
            "aria-label": "{title}",
            "data-testid": testid.unwrap_or_default(),

            // Header: title + dot tabs at top-right.
            div { class: "flex items-start justify-between gap-3",
                div { class: "min-w-0 flex-1",
                    h3 { class: "truncate {crate::TYPE_CARD_TITLE}", "{title}" }
                    if let Some(sub) = subtitle {
                        p { class: "mt-0.5 truncate text-[11px] text-zinc-400", "{sub}" }
                    }
                }
                div { class: "flex items-center gap-2 pt-0.5",
                    if let Some(action) = header_action {
                        {action}
                    }
                    DotTabBar {
                        tabs: tabs.clone(),
                        active: active_tab,
                        on_change: on_tab_change,
                    }
                }
            }

            // Tab content: 四个 panel 同格叠加，容器高度取最高者，切页签不跳动。
            // panel_3 缺省不渲染（≤3 tab 的卡），空槽位不占高度。
            div { class: "mt-3 grid grid-cols-1",
                div { class: "{c0}", {panel_0} }
                div { class: "{c1}", {panel_1} }
                div { class: "{c2}", {panel_2} }
                if let Some(p3) = panel_3 {
                    div { class: "{c3}", {p3} }
                }
            }
        }
    }
}
