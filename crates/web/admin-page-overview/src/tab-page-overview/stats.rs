//! 实时汇总统计区:统计卡网格 + 额度余量卡(编号段 3)。
//!
//! 纯展示:统计卡列表由页面从 `DashboardSummaryDto` 派生后传入;空/sparkline
//! 序列走等高占位(诚实降级),不在此层取数。

use dioxus::prelude::*;

use super::sparkline::Sparkline;
use crate::api;
use ui::components::card::Card;

/// Compact single-stat card occupying one grid column.
///
/// 统计卡挂 hoverable：悬停边框变亮是管理台统一交互（维护者拍板），
/// 原面板级 hover 位移/阴影装饰不回归（dsh 规格仅边框动态）；
/// `py-3!`/`gap-0!` 覆盖 Card 基串的 py-6/gap-6，保留原紧凑单行布局。
/// `sparkline` 传 `Some` 时在卡底渲染 12 点迷你面积线（空序列 → 等高占位）。
#[component]
pub fn StatCard(
    value: String,
    label: &'static str,
    #[props(default)] sparkline: Option<Vec<f64>>,
    /// SVG 渐变 id：同页多张 sparkline 各持一份，避免 `url(#id)` 串线。
    #[props(default)]
    gradient_id: &'static str,
) -> Element {
    rsx! {
        Card {
            hoverable: true,
            class: "cursor-default gap-0! px-4 py-3!",
            "data-testid": "{label}",
            p { class: "truncate text-base font-semibold text-foreground md:text-lg", "{value}" }
            p { class: "mt-0.5 truncate text-xs text-muted-foreground", "{label}" }
            if let Some(series) = sparkline {
                Sparkline { series, gradient_id }
            }
        }
    }
}

/// 第 7 张统计卡「额度余量」：剩余总额度折 $ 大数字 + runway 可用天数 + 口径小字。
///
/// runway = `quota_remaining / quota_today`（今日消耗速率），三态健康点
/// （阈值语义参照 deepdive §二-5）：
/// - `quota_today == 0` → 「无近期消耗」（灰点：无消耗速率，不做健康判断）；
/// - `remaining <= 0` → 「已耗尽」红点红字；
/// - runway < 3 天 → 黄点黄字（<1 天特判文案；≥999 天显示 999+ 天）；
/// - 其余 → 绿点。
#[component]
pub fn QuotaRemainingCard(remaining: i64, today: i64) -> Element {
    let (runway_line, runway_class, dot_class) = if today <= 0 {
        (
            "无近期消耗".to_string(),
            "text-muted-foreground".to_string(),
            "bg-zinc-500",
        )
    } else if remaining <= 0 {
        (
            "已耗尽".to_string(),
            "text-red-400".to_string(),
            "bg-red-500",
        )
    } else {
        let days = remaining as f64 / today as f64;
        let days_text = if days < 1.0 {
            "<1 天".to_string()
        } else if days >= 999.0 {
            "999+ 天".to_string()
        } else {
            format!("{days:.1} 天")
        };
        if days < 3.0 {
            (
                format!("可用 {days_text}"),
                "text-yellow-400".to_string(),
                "bg-yellow-400",
            )
        } else {
            (
                format!("可用 {days_text}"),
                "text-emerald-400".to_string(),
                "bg-emerald-400",
            )
        }
    };
    rsx! {
        Card {
            hoverable: true,
            class: "cursor-default gap-0! px-4 py-3!",
            "data-testid": "额度余量",
            div { class: "flex items-center gap-1.5",
                p { class: "truncate text-base font-semibold font-mono tabular-nums text-foreground md:text-lg", "{api::fmt_usd(remaining)}" }
                span { class: "h-2 w-2 shrink-0 rounded-full {dot_class}", aria_hidden: "true" }
            }
            p { class: "mt-0.5 truncate text-xs text-muted-foreground", "额度余量" }
            p {
                class: "mt-0.5 truncate text-xs font-medium {runway_class}",
                "data-testid": "额度余量可用天数",
                "{runway_line}"
            }
            p { class: "mt-1 text-[10px] leading-4 text-muted-foreground/70",
                "启用用户余额合计 ÷ 今日消耗"
            }
        }
    }
}
