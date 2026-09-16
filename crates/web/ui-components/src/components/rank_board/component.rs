//! RankBoard — 排行榜条目卡（读一次数据、按名次列条目的通用展示组件）。
//!
//! 从 admin-page-overview 真实用量榜的三张口径榜卡（tokens/调用数/费用）逐 DOM
//! 抽象而来：卡壳/条目几何不变，行内容全部由调用方预先格式化后经 [`RankRowView`]
//! 注入——本组件不做排序、不算口径、不知道「模型/额度」等业务概念。
//!
//! 结构约定：
//! - 卡壳复用 crate 内 [`crate::components::card::Card`](hoverable)——hover 仅边框
//!   变亮（secondary-hover token），与全站面板一致；Header/Title/Description 走
//!   shadcn 子件，内容区不再额外叠 padding。
//! - 条目双列摊开（`md:grid-cols-2`，移动端单列）；行 = 名次角标 + 名称 +
//!   右对齐数值 + 行内元信息（环比/份额）+ 比例条。条目无底色（维护者反馈：
//!   行内底色块很难看），呼吸感由 `gap-y-4` 行距承担。
//! - `rows` 的条数与顺序即展示顺序：调用方负责排序与截断（如只取前 10）。

use dioxus::prelude::*;

use crate::components::card::{Card, CardDescription, CardHeader, CardTitle};

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
/// * `testid` — 卡壳 data-testid（可空）。
#[component]
pub fn RankBoard(
    title: String,
    subtitle: String,
    rows: Vec<RankRowView>,
    footnote: Option<String>,
    testid: Option<String>,
) -> Element {
    rsx! {
        Card {
            hoverable: true,
            class: "p-5",
            "data-testid": testid.unwrap_or_default(),
            div { class: "space-y-4",
                CardHeader { class: "p-0",
                    CardTitle { class: "text-sm text-zinc-100", "{title}" }
                    CardDescription { class: "text-[11px]", "{subtitle}" }
                }
                div { class: "grid grid-cols-1 gap-x-6 gap-y-4 pt-1 md:grid-cols-2",
                    for r in rows {
                        div {
                            key: "{r.key}",
                            class: "flex items-center gap-2.5",
                            span { class: "flex h-5 w-5 shrink-0 items-center justify-center rounded bg-zinc-800/80 text-[10px] font-medium text-zinc-400 shadow-sm",
                                "{r.rank}"
                            }
                            div { class: "min-w-0 flex-1",
                                div { class: "flex items-center justify-between gap-3",
                                    span { class: "truncate text-xs font-medium text-zinc-200", "{r.name}" }
                                    div { class: "flex shrink-0 flex-col items-end gap-1",
                                        span { class: "font-mono text-xs font-semibold tabular-nums text-zinc-100",
                                            "{r.value}"
                                        }
                                        div { class: "flex items-center gap-1.5 text-[10px] leading-none",
                                            if let Some(m) = &r.meta {
                                                span { class: "font-medium tabular-nums {m.class}", "{m.label}" }
                                            }
                                            if let Some(s) = &r.share {
                                                span { class: "tabular-nums text-zinc-500", "{s}" }
                                            }
                                        }
                                    }
                                }
                                div { class: "mt-1.5 h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                                    div {
                                        class: "h-full rounded-full transition-all duration-300",
                                        style: "width: {r.bar_pct:.1}%; background: {r.bar_color}",
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(note) = footnote {
                    p { class: "border-t border-zinc-800/60 pt-2.5 text-[10px] text-zinc-500", "{note}" }
                }
            }
        }
    }
}
