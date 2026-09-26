//! RankBoard — 排行榜条目卡（读一次数据、按名次列条目的通用展示组件）。
//!
//! 从 admin-page-overview 真实用量榜的三张口径榜卡（tokens/调用数/费用）逐 DOM
//! 抽象而来：卡壳/条目几何不变，行内容全部由调用方预先格式化后经 [`RankRowView`]
//! 注入——本组件不做排序、不算口径、不知道「模型/额度」等业务概念。
//!
//! 结构约定：
//! - 卡壳复用 crate 内 [`crate::card::Card`](hoverable)——hover 仅边框
//!   变亮（secondary-hover token），与全站面板一致；Header/Title/Description 走
//!   shadcn 子件，内容区不再额外叠 padding。
//! - 条目单列排布（xl 三卡并排时卡内容区仅 ~320px，双列会把行挤到 ~158px/列）；
//!   行 = 名次角标 + 名称 + 右对齐数值 + 行内元信息（环比/份额）+ 比例条。条目
//!   无底色（维护者反馈：行内底色块很难看），呼吸感由 `gap-y-5` 行距承担。
//! - `rows` 的条数与顺序即展示顺序：调用方负责排序与截断（如只取前 10）。
//! - 来源契约：`bar_color` 必须来自调用方静态常量（品牌色表），`bar_pct` 渲染时
//!   钳制 0..=100；两者均不接受运行时用户输入，防止共享组件被误用为注入面。

use dioxus::prelude::*;

use crate::card::{Card, CardDescription, CardHeader, CardTitle};

/// 单个条目的行内元信息（环比标签等）。
///
/// `label` 为已格式化文本（如 `↑12%` / `↑new`），`class` 为其语义色
/// （如 `text-emerald-400`）；由调用方决定配色，组件不解读含义。
#[derive(Debug, Clone, PartialEq)]
pub struct RankRowMeta {
    /// 已格式化的标签文本（箭头已编入文本，保证 tabular 对齐）。
    pub label: String,
    /// 标签的语义色 class（emerald/rose/zinc 等）。
    pub class: &'static str,
}

/// 单个排行条目（全部为呈现态：调用方把业务行折算成这里的展示字段）。
#[derive(Debug, Clone, PartialEq)]
pub struct RankRowView {
    /// rsx key（通常是实体名，如同屏去重）。
    pub key: String,
    /// 展示名次（1 起）。角标文案由组件按 `1..=3` 强调色、其余中性渲染。
    pub rank: usize,
    /// 条目名（模型名/用户名等）。
    pub name: String,
    /// 主数值的已格式化文本（如 `2.0B` / `$41.80`）。
    pub value: String,
    /// 行内元信息（环比标签等，可空）。
    pub meta: Option<RankRowMeta>,
    /// 份额文本（如 `52.4%`，可空；与 meta 同行右侧并排）。
    pub share: Option<String>,
    /// 比例条宽度百分比（0.0..=100.0，调用方负责相对最大值的换算）。
    pub bar_pct: f64,
    /// 比例条颜色（css 颜色串，如品牌 hex；同榜多色由调用方排布）。
    pub bar_color: String,
}

/// 排行榜条目卡。
///
/// * `title` / `subtitle` — 卡头标题与一行口径说明。
/// * `rows` — 已排序、已格式化的条目（组件按给定顺序渲染）。
/// * `footnote` — 卡底口径小字（可空；有值时上边框分隔）。
/// * `testid` — 卡壳 data-testid（必填，供 UI 验证语义定位）。
#[component]
pub fn RankBoard(
    title: String,
    subtitle: String,
    rows: Vec<RankRowView>,
    footnote: Option<String>,
    testid: String,
) -> Element {
    rsx! {
        Card {
            hoverable: true,
            class: "p-5",
            "data-testid": "{testid}",
            div { class: "space-y-4",
                CardHeader { class: "p-0",
                    CardTitle { class: "{crate::T_text_sm} {crate::T_text_zinc_100}", "{title}" }
                    CardDescription { class: "{crate::T_text_11px}", "{subtitle}" }
                }
                // 单列条目(xl 三卡并排时卡内容区仅 ~320px,双列会把名字+数值+环比挤到
                // 158px/列,维护者多轮反馈的"间距没做好"根因在此);呼吸感由 gap-y-5 承担
                div { class: "grid grid-cols-1 gap-y-5 pt-1",
                    for r in rows {
                        div {
                            key: "{r.key}",
                            class: "flex items-center gap-3",
                            span { class: "flex h-5 w-5 shrink-0 items-center justify-center rounded bg-zinc-800/80 {crate::T_text_10px} {crate::T_font_medium} {crate::T_text_zinc_400} shadow-sm",
                                "{r.rank}"
                            }
                            div { class: "min-w-0 flex-1",
                                div { class: "flex items-center justify-between gap-3",
                                    span { class: "truncate {crate::T_text_xs} {crate::T_font_medium} {crate::T_text_zinc_200}", "{r.name}" }
                                    div { class: "flex shrink-0 flex-col items-end gap-1",
                                        span { class: "font-mono {crate::T_text_xs} {crate::T_font_semibold} tabular-nums {crate::T_text_zinc_100}",
                                            "{r.value}"
                                        }
                                        div { class: "flex items-center gap-1.5 {crate::T_text_10px} leading-none",
                                            if let Some(m) = &r.meta {
                                                span { class: "{crate::T_font_medium} tabular-nums {m.class}", "{m.label}" }
                                            }
                                            if let Some(s) = &r.share {
                                                span { class: "tabular-nums {crate::T_text_zinc_500}", "{s}" }
                                            }
                                        }
                                    }
                                }
                                div { class: "mt-2 h-1.5 w-full overflow-hidden rounded-full {crate::T_bg_zinc_800}",
                                    div {
                                        class: "h-full rounded-full transition-all duration-300",
                                        // bar_pct 钳到 0..=100;bar_color 只接受调用方静态
                                        // 常量(品牌 hex 表),不接受任何运行时用户输入
                                        style: "width: {r.bar_pct.clamp(0.0, 100.0):.1}%; background: {r.bar_color}",
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(note) = footnote {
                    p { class: "border-t border-zinc-800/60 pt-2.5 {crate::T_text_10px} {crate::T_text_zinc_500}", "{note}" }
                }
            }
        }
    }
}
