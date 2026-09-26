//! 实时汇总统计区:区块外壳 + 统计卡网格 + 额度余量卡(编号段 3)。
//!
//! 纯展示:统计卡列表由页面从 `DashboardSummaryDto` 派生后传入;空/sparkline
//! 序列走等高占位(诚实降级),不在此层取数。

use dioxus::prelude::*;

use super::shared::{
    LBL_QUOTA_REMAINING, QUOTA_FOOTNOTE, QuotaView, RUNWAY_AVAILABLE, RUNWAY_CAP_DAYS,
    RUNWAY_EXHAUSTED, RUNWAY_LT1_DAY, RUNWAY_NO_USAGE, RUNWAY_UNIT, SEC_STATS, STATS_LOADING,
    StatCardView, TESTID_QUOTA_RUNWAY,
};
use super::sparkline::Sparkline;
use crate::api;
use ui::card::Card;

/// 统计区外壳:区头(标题 + 右侧 asOf 裸本地时间) + 统计卡网格。
///
/// - 是什么:总览页第三区块的容器,一轮重构前整段 rsx 写在 page.rs 里。
/// - 负责什么:加载态占位 + 遍历渲染统计卡 + 尾部挂第 7 张额度余量卡。
/// - 交互逻辑:无事件、无状态;asOf 由页面解析后传入,解析失败传 `None` 即隐藏
///   (诚实降级,不写「数据截至」字样)。
/// - 样式:区头 `text-lg font-medium` 标题;网格 `grid-cols-1 sm:grid-cols-2
///   md:grid-cols-3 lg:grid-cols-5`(手机 1 栏 / 平板 3 栏 / Web 5 栏)。
/// - 数据流通:入参 `cards` 为页面派生好的统计卡视图,`quota` 为 `Some` 时渲染
///   第 7 张卡(拉取失败时页面传全零 DTO 的 QuotaView,保持卡位不变)。
#[component]
pub fn StatsSection(
    /// 汇总拉取中:dashed 骨架占位。
    loading: bool,
    /// 六张统计卡视图(含 sparkline 序列与渐变 id)。
    cards: Vec<StatCardView>,
    /// 第 7 张额度余量卡数据;`None` 时不渲染该卡。
    quota: Option<QuotaView>,
    /// 数据新鲜度本地时间;解析失败为 `None`。
    as_of: Option<String>,
) -> Element {
    rsx! {
        div { class: "space-y-3",
            div { class: "flex items-center justify-between",
                h2 { class: "text-lg font-medium text-foreground", "{SEC_STATS}" }
                div { class: "flex items-center gap-3",
                    // asOf 本地时间裸值(维护者要求:不写「数据截至」字样);
                    // 拉取失败时 summary 为 None,时间位自然隐藏(中性占位)。
                    if let Some(t) = as_of {
                        span {
                            class: "text-xs font-mono tabular-nums text-muted-foreground",
                            "data-testid": "overview-as-of",
                            "{t}"
                        }
                    }
                }
            }
            section { "data-testid": "overview-stats",
                class: "grid grid-cols-1 gap-3 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-5",
                if loading {
                    div { class: "col-span-full rounded-2xl border border-dashed border-border bg-card/50 py-10 text-center",
                        p { class: "text-muted-foreground", "{STATS_LOADING}" }
                    }
                } else {
                    for card in cards {
                        StatCard {
                            value: card.value,
                            label: card.label,
                            sparkline: card.sparkline,
                            gradient_id: card.gradient_id,
                        }
                    }
                    // 第 7 张卡:额度余量 + runway 可用天数(W1 后端已供 quotaRemaining);
                    // 拉取失败时页面传全零 DTO,$0.00 + 「无近期消耗」灰点。
                    if let Some(q) = quota {
                        QuotaRemainingCard { remaining: q.remaining, today: q.today }
                    }
                }
            }
        }
    }
}

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
            p { class: "truncate {ui::TYPE_TITLE} text-foreground md:text-lg", "{value}" }
            p { class: "mt-0.5 truncate {ui::TYPE_DESC} text-muted-foreground", "{label}" }
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
            RUNWAY_NO_USAGE.to_string(),
            "text-muted-foreground".to_string(),
            "bg-zinc-500",
        )
    } else if remaining <= 0 {
        (
            RUNWAY_EXHAUSTED.to_string(),
            "text-red-400".to_string(),
            "bg-red-500",
        )
    } else {
        let days = remaining as f64 / today as f64;
        let days_text = if days < 1.0 {
            RUNWAY_LT1_DAY.to_string()
        } else if days >= 999.0 {
            RUNWAY_CAP_DAYS.to_string()
        } else {
            format!("{days:.1}{RUNWAY_UNIT}")
        };
        if days < 3.0 {
            (
                format!("{RUNWAY_AVAILABLE}{days_text}"),
                "text-yellow-400".to_string(),
                "bg-yellow-400",
            )
        } else {
            (
                format!("{RUNWAY_AVAILABLE}{days_text}"),
                "text-emerald-400".to_string(),
                "bg-emerald-400",
            )
        }
    };
    rsx! {
        Card {
            hoverable: true,
            class: "cursor-default gap-0! px-4 py-3!",
            "data-testid": "{LBL_QUOTA_REMAINING}",
            div { class: "flex items-center gap-1.5",
                p { class: "truncate text-base font-semibold font-mono tabular-nums text-foreground md:text-lg", "{api::fmt_usd(remaining)}" }
                span { class: "h-2 w-2 shrink-0 rounded-full {dot_class}", aria_hidden: "true" }
            }
            p { class: "mt-0.5 truncate {ui::TYPE_DESC} text-muted-foreground", "{LBL_QUOTA_REMAINING}" }
            p {
                class: "mt-0.5 truncate {ui::TYPE_DESC} {runway_class}",
                "data-testid": "{TESTID_QUOTA_RUNWAY}",
                "{runway_line}"
            }
            p { class: "mt-1 {ui::TYPE_LABEL} leading-4 text-muted-foreground/70",
                "{QUOTA_FOOTNOTE}"
            }
        }
    }
}
