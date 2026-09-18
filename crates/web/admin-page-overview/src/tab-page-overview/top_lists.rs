//! 消耗前十模型/用户榜:行视图类型 + 行组件(编号段 4)。

use dioxus::prelude::*;

use crate::api;

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
