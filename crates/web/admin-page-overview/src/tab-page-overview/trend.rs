//! 用量趋势大面板:标题/时间窗切换 + 三态分支 + 直方图/汇总/悬浮卡组合(编号段 1)。
//!
//! - 是什么:总览页第一个区块,展示窗口内按模型堆叠的 tokens 用量趋势。
//! - 负责什么:持有悬浮卡状态、算 Y 轴封顶与峰值/均值/占比,并把已整形的数据
//!   交给 [`TrendHistogram`](super::histogram::TrendHistogram) 与
//!   [`TrendSummary`](super::summary::TrendSummary) 渲染;本文件不含渲染细节。
//! - 交互逻辑:时间窗切换写回页面持有的 `timeframe` signal(页面 effect 重拉);
//!   悬浮卡状态是本面板唯一的本地状态(`tip`),由直方图写入、[`TrendTipCard`]
//!   消费。
//! - 样式:zinc-900 卡底 + hover 边框变亮;四态分支 —— 失败红盒 / 加载 dashed
//!   骨架 / 空窗诚实空态 / 正常图表。
//! - 数据流通:入参 `buckets` / `model_order` 为页面已 pivot 的窗口数据;
//!   `empty_window` 由页面按「每桶 total 均为 0」判定后传入,避免两处判空错位。

use dioxus::prelude::*;

use super::histogram::TrendHistogram;
use super::shared::{
    SEC_TREND, TREND_EMPTY, TREND_EMPTY_HINT, TREND_ERR, TREND_LOADING, TREND_UNIT, TrendTip,
};
use super::summary::{TopModelRow, TrendSummary};
use super::tooltip::TrendTipCard;
use crate::api::{self, TrendBucketFE};
use crate::shared::{MODEL_COLORS, TimeframeTabs, fmt_raw};

/// 用量趋势大面板: 左侧累加直方图(按模型堆叠) + 右侧数据位。
/// 时间窗: 今天(24h 逐时)/本周(7d 逐天)/本月(30d 逐天)/今年(12mo 逐月)。
/// 数据来自真实 /api/log/trend 聚合;窗口内无调用时显示诚实空态。
#[component]
pub fn TrendPanel(
    timeframe: Signal<&'static str>,
    buckets: Signal<Vec<TrendBucketFE>>,
    model_order: Signal<Vec<String>>,
    loading: bool,
    err: Option<String>,
    empty_window: bool,
) -> Element {
    let tf = timeframe();
    let all_buckets = buckets();
    let names = model_order();
    // 窗口内全部为空桶时,直方图没有意义 → 诚实空态
    let has_any = all_buckets.iter().any(|b| b.total > 0.0);

    let total_all: f64 = all_buckets.iter().map(|b| b.total).sum();
    let max_total = all_buckets
        .iter()
        .map(|b| b.total)
        .fold(0.0f64, f64::max)
        .max(1.0);
    // Y 轴封顶: step = ⌈max/4 的最高位⌉, 轴顶 = 4×step — 最高柱恒低于顶格,
    // 5 条虚线 (含 0) 等间隔且刻度整齐 (参照 new-api VChart 的 nice ticks)。
    let axis_max = api::nice_axis_max(max_total);
    let avg = total_all / all_buckets.len().max(1) as f64;
    let peak = all_buckets
        .iter()
        .max_by(|a, b| a.total.partial_cmp(&b.total).unwrap())
        .cloned()
        .unwrap_or_default();

    let mut per_model_tot = vec![0.0f64; names.len()];
    for b in &all_buckets {
        for (i, v) in b.per_model.iter().enumerate() {
            per_model_tot[i] += v;
        }
    }
    let mut order: Vec<usize> = (0..names.len()).collect();
    order.sort_by(|&a, &b| per_model_tot[b].partial_cmp(&per_model_tot[a]).unwrap());
    order.truncate(5);
    // Top5 图例: 名称 + 段色(与直方图同下标同色) + 占窗口总量百分比
    let top_models: Vec<TopModelRow> = order
        .iter()
        .map(|&i| {
            (
                names[i].clone(),
                MODEL_COLORS[i % MODEL_COLORS.len()],
                per_model_tot[i] / total_all * 100.0,
            )
        })
        .collect();

    // 共享悬浮卡: 整列模式列明细, 色块模式列单模型。fixed 定位不受滚动影响。
    // 只在本面板读取渲染,写入由 TrendHistogram 的鼠标事件完成。
    let tip = use_signal(|| None::<TrendTip>);

    rsx! {
        section { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-5 transition-all duration-300 hover:border-zinc-700",
            div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                div {
                    h2 { class: "text-sm font-medium text-zinc-300", "{SEC_TREND}" }
                    // 时间窗动态副标题:与 window_start 的窗口语义一致(今天=24 小时桶/本周=7 天桶/本月=30 天桶/今年=12 月桶)
                    p { class: "mt-0.5 text-xs text-zinc-500", "data-testid": "trend-window-caption", "{api::window_caption(tf)}" }
                }
                div { class: "flex items-center gap-4",
                    div { class: "text-right",
                        p { class: "text-2xl font-semibold leading-none text-zinc-100", "{fmt_raw(total_all as i64)}" }
                        p { class: "mt-1 text-[11px] text-zinc-600", "{TREND_UNIT}" }
                    }
                    // 时间窗切换胶囊(与排行榜共用组件);总览趋势面板无 testid 前缀
                    TimeframeTabs { timeframe }
                }
            }
            if let Some(e) = err {
                div { class: "rounded-xl border border-red-800/60 bg-red-950/40 px-4 py-8 text-center",
                    p { class: "text-sm text-red-300", "{TREND_ERR}" }
                    p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                }
            } else if loading {
                div { class: "rounded-xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "{TREND_LOADING}" }
                }
            } else if !has_any || empty_window {
                div { class: "rounded-xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "{TREND_EMPTY}" }
                    p { class: "mt-1 text-xs text-zinc-600", "{TREND_EMPTY_HINT}" }
                }
            } else {
                div { class: "grid grid-cols-1 gap-5 xl:grid-cols-3",
                    // 左: 累加直方图(带横向虚网格 + 两级悬浮卡)
                    TrendHistogram {
                        buckets: all_buckets.clone(),
                        names: names.clone(),
                        axis_max,
                        tip,
                    }
                    // 右: 数据位 + Top5 图例 (标准深灰弱边框)
                    TrendSummary {
                        peak_label: peak.label.clone(),
                        peak_total: peak.total,
                        avg,
                        model_count: names.len(),
                        total: total_all,
                        top_models,
                    }
                }
                // 悬浮卡: 整列全模型分解 / 单段位详情(fixed 定位, 不被裁剪)
                if let Some(t) = tip() {
                    TrendTipCard { tip: t }
                }
            }
        }
    }
}
