//! 用量趋势大面板:时间窗切换 + 按模型堆叠直方图 + 右侧数据位 + 两级悬浮卡
//! (编号段 1)。数据来自真实 `/api/log/trend` 聚合。

use dioxus::prelude::*;

use crate::api::{self, TrendBucketFE};
use crate::shared::{MODEL_COLORS, fmt_raw};

/// 趋势图悬浮卡: 整列分解 / 单色块详情
#[derive(Clone)]
enum TrendTip {
    /// x, y, 桶标签, 明细(名称, 颜色, 值), 桶总量
    Column(f64, f64, String, Vec<(String, &'static str, f64)>, f64),
    /// x, y, 桶标签, 模型名, 颜色, 值
    Segment(f64, f64, String, String, &'static str, f64),
}

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

    // 共享悬浮卡: 整列模式列明细, 色块模式列单模型。fixed 定位不受滚动影响。
    let mut tip = use_signal(|| None::<TrendTip>);

    rsx! {
        section { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-5 transition-all duration-300 hover:border-zinc-700",
            div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                div {
                    h2 { class: "text-sm font-medium text-zinc-300", "用量趋势" }
                    // 时间窗动态副标题:与 window_start 的窗口语义一致(今天=24 小时桶/本周=7 天桶/本月=30 天桶/今年=12 月桶)
                    p { class: "mt-0.5 text-xs text-zinc-500", "data-testid": "trend-window-caption", "{api::window_caption(tf)}" }
                }
                div { class: "flex items-center gap-4",
                    div { class: "text-right",
                        p { class: "text-2xl font-semibold leading-none text-zinc-100", "{fmt_raw(total_all as i64)}" }
                        p { class: "mt-1 text-[11px] text-zinc-600", "tokens(合计)" }
                    }
                    div { class: "flex items-center gap-1.5 rounded-lg border border-zinc-800 bg-zinc-950 p-1",
                        for t in ["今天", "本周", "本月", "今年"] {
                            button {
                                class: "rounded-md px-2.5 py-1 text-xs font-medium transition-colors",
                                class: if tf == t { "bg-zinc-800 text-zinc-100 shadow-sm" } else { "text-zinc-400 hover:text-zinc-200" },
                                onclick: move |_| timeframe.set(t),
                                "{t}"
                            }
                        }
                    }
                }
            }
            if let Some(e) = err {
                div { class: "rounded-xl border border-red-800/60 bg-red-950/40 px-4 py-8 text-center",
                    p { class: "text-sm text-red-300", "加载趋势失败" }
                    p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                }
            } else if loading {
                div { class: "rounded-xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "正在加载趋势…" }
                }
            } else if !has_any || empty_window {
                div { class: "rounded-xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "该时间窗内暂无调用数据" }
                    p { class: "mt-1 text-xs text-zinc-600", "发起一次 /v1 调用后这里会展示真实用量" }
                }
            } else {
                div { class: "grid grid-cols-1 gap-5 xl:grid-cols-3",
                    // 左: 累加直方图(带横向虚网格 + 两级悬浮卡)
                    div {
                        class: "xl:col-span-2",
                        onmouseleave: move |_| tip.set(None),
                        div { class: "relative",
                            div { class: "pointer-events-none absolute inset-0 flex flex-col justify-between py-0", aria_hidden: "true",
                                // 顶格是封顶线 (axis_max), 其下三条是 step 等分, 最后是 0 基线
                                for i in [4, 3, 2, 1] {
                                    div { class: "relative w-full border-t border-dashed border-zinc-800",
                                        span { class: "absolute -top-2 right-0 text-[10px] text-zinc-600", "{fmt_raw((axis_max * i as f64 / 4.0) as i64)}" }
                                    }
                                }
                                div { class: "relative w-full border-t border-dashed border-zinc-800",
                                    span { class: "absolute -top-2 right-0 text-[10px] text-zinc-600", "0" }
                                }
                            }
                            div { class: "relative flex h-56 items-end", style: "gap: 3px",
                                for b in all_buckets.iter() {
                                    {
                                        let hpct = (b.total / axis_max * 100.0).max(3.0);
                                        let label = b.label.clone();
                                        // 列模式明细: 排序(值降序) + Total + 超 10 行折叠「+N more」
                                        // —— 纯整形逻辑收在 api::trend_column_tip(可单测),渲染层只消费结果
                                        let col_tip = api::trend_column_tip(&b.per_model, &names, &MODEL_COLORS);
                                        let col_rows = col_tip.rows;
                                        let col_total = col_tip.total;
                                        rsx! {
                                            div {
                                                class: "group relative flex h-full flex-1 cursor-default flex-col justify-end",
                                                onmouseleave: move |_| tip.set(None),
                                                // 悬停整列: 通顶全高半透明背景光柱
                                                div { class: "pointer-events-none absolute inset-x-0 top-0 bottom-0 rounded-sm transition-colors duration-150 group-hover:bg-zinc-100/10" }

                                                // 顶层无色透明区: 悬停时显示该列全部明细
                                                div {
                                                    class: "pointer-events-auto flex-1 w-full cursor-pointer",
                                                    onmouseenter: {
                                                        let c_label = label.clone();
                                                        let c_rows = col_rows.clone();
                                                        move |evt| {
                                                            let p = evt.data.client_coordinates();
                                                            tip.set(Some(TrendTip::Column(p.x, p.y, c_label.clone(), c_rows.clone(), col_total)));
                                                        }
                                                    },
                                                    onmousemove: {
                                                        let c_label = label.clone();
                                                        let c_rows = col_rows.clone();
                                                        move |evt| {
                                                            let p = evt.data.client_coordinates();
                                                            tip.set(Some(TrendTip::Column(p.x, p.y, c_label.clone(), c_rows.clone(), col_total)));
                                                        }
                                                    },
                                                }

                                                // 直方图有色堆叠容器 (堆叠段连续无间隙, 参照 new-api 堆叠图;
                                                // 之前 gap 1.5px 让亚像素小片段看起来像"间隙", 视觉不一致)
                                                div {
                                                    class: "relative flex w-full flex-col-reverse overflow-hidden rounded-[3px] transition-all duration-200",
                                                    style: "height: {hpct:.1}%",
                                                    for (i, v) in b.per_model.iter().enumerate() {
                                                        {
                                                            let seg_label_1 = b.label.clone();
                                                            let seg_name_1 = names[i].clone();
                                                            let seg_label_2 = b.label.clone();
                                                            let seg_name_2 = names[i].clone();
                                                            let seg_color = MODEL_COLORS[i % MODEL_COLORS.len()];
                                                            let seg_val = *v;
                                                            let denom = if b.total > 0.0 { b.total } else { 1.0 };
                                                            rsx! {
                                                                div {
                                                                    class: "pointer-events-auto w-full cursor-pointer hover:brightness-125 transition-all",
                                                                    style: "height: {(v / denom * 100.0):.1}%; background: {seg_color}",
                                                                    onmouseenter: move |evt| {
                                                                        evt.stop_propagation();
                                                                        let p = evt.data.client_coordinates();
                                                                        tip.set(Some(TrendTip::Segment(p.x, p.y, seg_label_1.clone(), seg_name_1.clone(), seg_color, seg_val)));
                                                                    },
                                                                    onmousemove: move |evt| {
                                                                        evt.stop_propagation();
                                                                        let p = evt.data.client_coordinates();
                                                                        tip.set(Some(TrendTip::Segment(p.x, p.y, seg_label_2.clone(), seg_name_2.clone(), seg_color, seg_val)));
                                                                    },
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "mt-2 flex text-[10px] text-zinc-600", style: "gap: 3px",
                            for b in all_buckets.iter() {
                                span { class: "flex-1 truncate text-center",
                                    if b.show_label { "{b.label}" }
                                }
                            }
                        }
                    }
                    // 右: 数据位 + Top5 图例 (标准深灰弱边框)
                    div { class: "flex flex-col justify-between gap-5 rounded-xl border border-zinc-800 bg-zinc-900/50 p-5",
                        div { class: "grid grid-cols-2 gap-3",
                            div {
                                p { class: "text-[11px] text-zinc-600", "峰值桶" }
                                p { class: "mt-1 truncate text-sm font-semibold text-zinc-100", "{peak.label}" }
                                p { class: "text-xs font-mono text-zinc-500", "{fmt_raw(peak.total as i64)}" }
                            }
                            div {
                                p { class: "text-[11px] text-zinc-600", "平均每桶" }
                                p { class: "mt-1 text-sm font-semibold text-zinc-100", "{fmt_raw(avg as i64)}" }
                                p { class: "text-xs font-mono text-zinc-500", "均值" }
                            }
                            div {
                                p { class: "text-[11px] text-zinc-600", "活跃模型" }
                                p { class: "mt-1 text-sm font-semibold text-zinc-100", "{names.len()} 个" }
                                p { class: "text-xs font-mono text-zinc-500", "窗口内有调用" }
                            }
                            div {
                                p { class: "text-[11px] text-zinc-600", "区间总量" }
                                p { class: "mt-1 text-sm font-semibold text-zinc-100", "{fmt_raw(total_all as i64)}" }
                                p { class: "text-xs font-mono text-zinc-500", "tokens" }
                            }
                        }
                        div { class: "border-t border-zinc-800/80 pt-3",
                            p { class: "mb-2 text-[11px] font-medium text-zinc-500", "主力模型 Top5" }
                            for &i in order.iter() {
                                div { class: "flex items-center gap-2 py-1 text-xs",
                                    span { class: "h-2 w-2 shrink-0 rounded-sm", style: "background: {MODEL_COLORS[i % MODEL_COLORS.len()]}" }
                                    span { class: "flex-1 truncate text-zinc-300", "{names[i]}" }
                                    span { class: "font-mono text-zinc-500", "{per_model_tot[i] / total_all * 100.0:.1}%" }
                                }
                            }
                        }
                    }
                }
                // 悬浮卡: 整列全模型分解 / 单段位详情(fixed 定位, 不被裁剪)
                if let Some(t) = tip() {
                    match t {
                        TrendTip::Column(x, y, label, rows, total) => rsx! {
                            TrendTooltipContainer { x, y, label,
                                div { class: "mb-2.5 flex items-center justify-between border-b border-zinc-800/80 pb-2 text-xs text-zinc-400",
                                    span { "Total" }
                                    span { class: "font-mono font-semibold text-zinc-100", "{fmt_raw(total as i64)}" }
                                }
                                div { class: "flex flex-col gap-1.5",
                                    for (name, color, v) in rows.iter() {
                                        div { class: "flex items-center justify-between gap-4 text-xs",
                                            div { class: "flex items-center min-w-0",
                                                span { class: "h-2 w-2 shrink-0 rounded-[2px]", style: "background: {color}" }
                                                span { class: "truncate text-zinc-300 ml-2", "{name}" }
                                            }
                                            span { class: "shrink-0 font-mono font-medium text-zinc-100", "{fmt_raw(*v as i64)}" }
                                        }
                                    }
                                }
                            }
                        },
                        TrendTip::Segment(x, y, label, name, color, v) => rsx! {
                            TrendTooltipContainer { x, y, label,
                                div { class: "flex items-center justify-between gap-4 text-xs",
                                    div { class: "flex items-center min-w-0",
                                        span { class: "h-2.5 w-2.5 shrink-0 rounded-[2px]", style: "background: {color}" }
                                        span { class: "font-medium text-zinc-200 ml-2", "{name}" }
                                    }
                                    span { class: "shrink-0 font-mono font-bold text-zinc-100", "{fmt_raw(v as i64)}" }
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}

/// 悬浮卡通用外框组件，保持单色块和整列的阴影、圆角、背景和内边距完全统一
#[component]
fn TrendTooltipContainer(x: f64, y: f64, label: String, children: Element) -> Element {
    // 智能定位：
    // 1. 水平翻转：x > 260px 时往左侧展开，否则往右侧展开，防止在手机与窄屏边缘被右边框裁切。
    // 2. 垂直修正：避免被顶部导航遮挡。
    let transform = if x > 260.0 {
        "translate(calc(-100% - 12px), -50%)"
    } else {
        "translate(12px, -50%)"
    };
    let clamped_y = y.max(80.0);
    let opacity_class = if x == 0.0 && y == 0.0 {
        "opacity-0 scale-95"
    } else {
        "opacity-100 scale-100"
    };
    rsx! {
        div {
            class: "pointer-events-none fixed z-50 rounded-xl border border-border/80 bg-card/95 p-3 text-xs shadow-2xl backdrop-blur-md transition-all duration-150 ease-out {opacity_class} max-sm:left-3! max-sm:right-3! max-sm:bottom-4! max-sm:top-auto! max-sm:transform-none! max-sm:w-auto!",
            style: "left: {x}px; top: {clamped_y}px; transform: {transform}; max-width: calc(100vw - 24px);",
            p { class: "mb-1.5 text-xs font-semibold text-muted-foreground", "{label}" }
            {children}
        }
    }
}
