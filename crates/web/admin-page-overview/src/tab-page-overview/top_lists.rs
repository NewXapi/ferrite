//! 消耗前十模型/用户榜:行视图类型 + 行组件 + 榜单卡外壳(编号段 4)。
//!
//! `TopListCard` 是模型榜与用户榜共用的卡外壳(一轮重构前两处在 page.rs 里各写
//! 一份 rsx),差异只在标题、合计口径与 testid,故全部由 props 注入。

use dioxus::prelude::*;

use super::shared::{TOP_EMPTY, TOP_FOOTNOTE};
use crate::api;
use ui::components::card::{Card, CardContent, CardHeader};

/// Top 榜单行渲染所需的前端视图（在 use_effect 内从 `UsageTopRow` 整形一次）。
#[derive(Clone, PartialEq)]
pub struct TopRowFE {
    pub name: String,
    /// 展示值：用户榜 `fmt_usd(quota)`、模型榜 `fmt_raw(tokens)`。
    pub amount: String,
    /// 行值占前 10 名合计的份额（合计 ≤ 0 时为 `None`，不显示）。
    pub share: Option<String>,
    /// 本窗 tokens 环比上一等长窗口的增长率（本窗为 0 时为 `None`，不显示）。
    pub growth: Option<api::Growth>,
}

/// 消耗前十榜卡片外壳:卡头(标题 + 右对齐合计) + 卡体(空态 / 行列表 + 脚注)。
///
/// - 是什么:模型榜与用户榜共用的容器(两榜唯一差异是合计口径与 testid)。
/// - 负责什么:只渲染;行视图由页面在 effect 内整形好后传入。
/// - 交互逻辑:无事件、无状态(纯展示)。
/// - 样式:面板卡挂 hoverable(悬停边框变亮的动态全站回归);调用方
///   `py-0!/gap-0!/px-4!/py-3!/p-4!/pb-3!` 覆盖 Card 基串的 py-6/gap-6/px-6,
///   尾缀 `!` 确保压过 Tailwind 同属性工具类,保留原紧凑条头布局。
/// - 数据流通:入参 `rows` 为已格式化的行视图;对外无回写。
#[component]
pub fn TopListCard(
    /// 卡头标题(如「消耗前十模型」)。
    title: &'static str,
    /// 卡头右侧合计大数字(模型榜 `fmt_raw(tokens)`、用户榜 `fmt_usd(quota)`)。
    total_text: String,
    /// 合计单位小字(如「tokens 合计」/「$ 合计」)。
    total_label: &'static str,
    /// 合计块的 `data-testid`。
    total_testid: &'static str,
    /// 榜单行(空则渲染诚实空态)。
    rows: Vec<TopRowFE>,
) -> Element {
    rsx! {
        Card {
            hoverable: true,
            class: "gap-0! overflow-hidden py-0!",
            CardHeader {
                class: "border-b border-border/50 px-4! py-3!",
                div { class: "flex items-center justify-between gap-3",
                    h3 { class: "text-sm font-medium text-foreground", "{title}" }
                    div { class: "text-right", "data-testid": "{total_testid}",
                        p { class: "text-sm font-semibold font-mono tabular-nums text-foreground", "{total_text}" }
                        p { class: "text-[10px] text-muted-foreground", "{total_label}" }
                    }
                }
            }
            CardContent {
                class: "flex-1 space-y-3 p-4! pb-3!",
                if rows.is_empty() {
                    p { class: "py-6 text-center text-xs text-muted-foreground", "{TOP_EMPTY}" }
                }
                for (i, row) in rows.iter().enumerate() {
                    TopRowItem { index: i, name: row.name.clone(), amount: row.amount.clone(), growth: row.growth.clone(), share: row.share.clone() }
                }
                p { class: "pt-1 text-[10px] leading-4 text-muted-foreground/60",
                    "{TOP_FOOTNOTE}"
                }
            }
        }
    }
}

/// Top 榜单行：名次 + 名称 + 右侧「数值在上、增长率与份额在下」双行列对齐。
#[component]
pub fn TopRowItem(
    index: usize,
    name: String,
    amount: String,
    growth: Option<api::Growth>,
    share: Option<String>,
) -> Element {
    rsx! {
        div { class: "flex items-center gap-3 rounded-lg -mx-2 px-2 py-1.5 transition-all hover:bg-accent cursor-default",
            div { class: "flex h-5 w-5 shrink-0 items-center justify-center rounded bg-secondary/80 text-[10px] font-medium text-muted-foreground shadow-sm transition-colors hover:bg-accent hover:text-foreground", "{index + 1}" }
            div { class: "flex-1 min-w-0 flex items-center justify-between gap-3",
                span { class: "truncate text-sm font-medium text-foreground/80 transition-colors hover:text-foreground", "{name}" }
                div { class: "flex shrink-0 flex-col items-end gap-0.5",
                    span { class: "text-xs font-mono text-muted-foreground transition-colors hover:text-foreground/80", "{amount}" }
                    div { class: "flex items-center gap-1.5 text-[10px] leading-none",
                        if let Some(g) = growth {
                            span { class: "font-medium tabular-nums {g.text_class()}", "{g.label()}" }
                        }
                        if let Some(s) = share {
                            span { class: "text-muted-foreground/70 tabular-nums", "{s}" }
                        }
                    }
                }
            }
        }
    }
}
