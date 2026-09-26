//! 趋势面板右栏:四宫格数据位 + 主力模型 Top5 图例。
//!
//! - 是什么:趋势面板右栏(`xl` 三列布局占 1 列)的汇总区,含峰值桶 / 平均每桶 /
//!   活跃模型 / 区间总量四个数据位,下方是窗口内消耗占比前五的模型图例。
//! - 负责什么:只做展示;四个数据位与 Top5 的排序、占比全部由面板算好传入。
//! - 交互逻辑:纯展示,无事件、无状态。
//! - 样式:标准深灰弱边框卡(`border-border bg-card/50 p-5`),数据位
//!   `grid-cols-2 gap-3`,图例色点与直方图段色同源(闭环图例)。
//! - 数据流通:入参全部为已格式化的标量/`Vec`,对外无回写。

use dioxus::prelude::*;

use super::shared::{
    TREND_AVG, TREND_AVG_SUB, TREND_MODELS, TREND_MODELS_SUB, TREND_MODELS_UNIT, TREND_PEAK,
    TREND_RANGE_SUB, TREND_RANGE_TOTAL, TREND_TOP5,
};
use crate::shared::fmt_raw;

/// 主力模型图例行:模型名 + 段色 + 占窗口总量的百分比。
pub type TopModelRow = (String, &'static str, f64);

/// 趋势面板右栏汇总区。
#[component]
pub fn TrendSummary(
    /// 峰值桶标签(桶名,由面板从 buckets 取 max)。
    peak_label: String,
    /// 峰值桶 tokens。
    peak_total: f64,
    /// 平均每桶 tokens。
    avg: f64,
    /// 窗口内有调用的模型数。
    model_count: usize,
    /// 窗口内 tokens 合计。
    total: f64,
    /// 主力模型 Top5:(名称, 段色, 占窗口总量百分比)。
    top_models: Vec<TopModelRow>,
) -> Element {
    rsx! {
        div { class: "flex flex-col justify-between gap-5 rounded-xl border border-border bg-card/50 p-5",
            div { class: "grid grid-cols-2 gap-3",
                div {
                    p { class: "{ui::TYPE_LABEL}", "{TREND_PEAK}" }
                    p { class: "mt-1 truncate {ui::TYPE_CARD_TITLE}", "{peak_label}" }
                    p { class: "text-xs font-mono text-muted-foreground", "{fmt_raw(peak_total as i64)}" }
                }
                div {
                    p { class: "{ui::TYPE_LABEL}", "{TREND_AVG}" }
                    p { class: "mt-1 {ui::TYPE_CARD_TITLE}", "{fmt_raw(avg as i64)}" }
                    p { class: "text-xs font-mono text-muted-foreground", "{TREND_AVG_SUB}" }
                }
                div {
                    p { class: "{ui::TYPE_LABEL}", "{TREND_MODELS}" }
                    p { class: "mt-1 {ui::TYPE_CARD_TITLE}", "{model_count}{TREND_MODELS_UNIT}" }
                    p { class: "text-xs font-mono text-muted-foreground", "{TREND_MODELS_SUB}" }
                }
                div {
                    p { class: "{ui::TYPE_LABEL}", "{TREND_RANGE_TOTAL}" }
                    p { class: "mt-1 {ui::TYPE_CARD_TITLE}", "{fmt_raw(total as i64)}" }
                    p { class: "text-xs font-mono text-muted-foreground", "{TREND_RANGE_SUB}" }
                }
            }
            div { class: "border-t border-border/80 pt-3",
                p { class: "mb-2 {ui::TYPE_LABEL}", "{TREND_TOP5}" }
                for (name, color, pct) in top_models.iter() {
                    div { class: "flex items-center gap-2 py-1 {ui::TYPE_DESC}",
                        span { class: "h-2 w-2 shrink-0 rounded-sm", style: "background: {color}" }
                        span { class: "flex-1 truncate text-foreground", "{name}" }
                        span { class: "font-mono text-muted-foreground", "{pct:.1}%" }
                    }
                }
            }
        }
    }
}
