use dioxus::prelude::*;

use crate::api;

// Layout convention (共享给所有面板组件, 详见仓库 README.md):
//   页面网格  `grid-cols-1 md:grid-cols-3 xl:grid-cols-5`  —— 手机 1 栏 / 平板 3 栏 / Web 5 栏。
//   小卡片(统计卡)占 1 栏; 宽面板并排: 热力图类 `md:col-span-2 xl:col-span-3`,
//   列表/分布类 `md:col-span-1 xl:col-span-2`; 手机端一律堆叠, 定宽内容用横向滚动。

/// Overview canvas (总览): responsive odd-column grid — 2 cols on mobile,
/// 3 on md, 5 on xl; wide sections span every column.
/// 数据经 `api` 取用;grayscale only.
#[component]
pub fn OverviewPanel() -> Element {
    // ponytail: full UI overhaul to add breakdown cards for all timeframes in one go.
    let timeframe = use_signal(|| "今天"); // "今天", "本周", "本月", "今年"

    // Get the stats for the currently selected timeframe
    let (stats, user_stats, model_stats) = api::overview::fetch_timeframe_stats(timeframe());

    rsx! {
        div { class: "flex flex-col gap-3 p-4 md:gap-4 md:p-6",
            // 用量趋势大面板(含时间窗切换)
            TrendPanel { timeframe }

            // 模型调用健康度统计卡片
            crate::health::HealthStats {}

            // Top-level stats
            section { class: "grid grid-cols-2 gap-3 md:grid-cols-4 xl:grid-cols-5",
                for &(value, label) in stats.iter() {
                    StatCard { value, label }
                }
            }

            // 模型健康度月份分布横线
            crate::health::HealthMixer {}

            // Top 10 breakdowns
            section { class: "grid grid-cols-1 gap-3 md:grid-cols-2 lg:gap-4",
                // Top 10 Models
                div { class: "rounded-xl border border-zinc-800 bg-zinc-900 overflow-hidden flex flex-col transition-all duration-300 hover:border-zinc-700 hover:shadow-lg hover:shadow-black/20 group",
                    div { class: "border-b border-zinc-800/50 bg-zinc-900/50 px-4 py-3 transition-colors group-hover:bg-zinc-800/20",
                        h3 { class: "text-sm font-medium text-zinc-100", "消耗前十模型" }
                    }
                    div { class: "p-4 space-y-3 flex-1",
                        for (i, &(name, amount, pct)) in model_stats.iter().enumerate() {
                            div { class: "flex items-center gap-3 rounded-lg -mx-2 px-2 py-1.5 transition-all hover:bg-zinc-800/60 cursor-default",
                                div { class: "flex h-5 w-5 shrink-0 items-center justify-center rounded bg-zinc-800/80 text-[10px] font-medium text-zinc-400 shadow-sm transition-colors hover:bg-zinc-700 hover:text-zinc-200", "{i + 1}" }
                                div { class: "flex-1 min-w-0 flex items-center justify-between",
                                    span { class: "truncate text-sm font-medium text-zinc-300 transition-colors hover:text-zinc-100", "{name}" }
                                    div { class: "flex items-center gap-3",
                                        span { class: "text-xs font-mono text-zinc-500 transition-colors hover:text-zinc-300", "{amount}" }
                                        span { class: "w-10 text-right text-xs text-zinc-500 font-medium", "{pct:.1}%" }
                                    }
                                }
                            }
                        }
                    }
                }

                // Top 10 Users
                div { class: "rounded-xl border border-zinc-800 bg-zinc-900 overflow-hidden flex flex-col transition-all duration-300 hover:border-zinc-700 hover:shadow-lg hover:shadow-black/20 group",
                    div { class: "border-b border-zinc-800/50 bg-zinc-900/50 px-4 py-3 transition-colors group-hover:bg-zinc-800/20",
                        h3 { class: "text-sm font-medium text-zinc-100", "消耗前十用户" }
                    }
                    div { class: "p-4 space-y-3 flex-1",
                        for (i, &(name, amount, pct)) in user_stats.iter().enumerate() {
                            div { class: "flex items-center gap-3 rounded-lg -mx-2 px-2 py-1.5 transition-all hover:bg-zinc-800/60 cursor-default",
                                div { class: "flex h-5 w-5 shrink-0 items-center justify-center rounded bg-zinc-800/80 text-[10px] font-medium text-zinc-400 shadow-sm transition-colors hover:bg-zinc-700 hover:text-zinc-200", "{i + 1}" }
                                div { class: "flex-1 min-w-0 flex items-center justify-between",
                                    span { class: "truncate text-sm font-medium text-zinc-300 transition-colors hover:text-zinc-100", "{name}" }
                                    div { class: "flex items-center gap-3",
                                        span { class: "text-xs font-mono text-zinc-500 transition-colors hover:text-zinc-300", "{amount}" }
                                        span { class: "w-10 text-right text-xs text-zinc-500 font-medium", "{pct:.1}%" }
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

/// Compact single-stat card occupying one grid column.
#[component]
fn StatCard(value: &'static str, label: &'static str) -> Element {
    rsx! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-900 px-4 py-3 transition-all duration-200 hover:border-zinc-700 hover:bg-zinc-900/80 hover:-translate-y-0.5 hover:shadow-md hover:shadow-black/20 group cursor-default",
            p { class: "truncate text-base font-semibold text-zinc-100 transition-colors group-hover:text-white md:text-lg", "{value}" }
            p { class: "mt-0.5 truncate text-xs text-zinc-500 transition-colors group-hover:text-zinc-400", "{label}" }
        }
    }
}

/// 模型配色(内联 hex, 不走 Tailwind 扫描, 避免 @source 漏扫隐形)
const MODEL_COLORS: [&str; 10] = [
    "#3b82f6", "#c4b5fd", "#a78bfa", "#facc15", "#fb8500", "#34d399", "#22d3ee", "#f472b6",
    "#a3e635", "#a1a1aa",
];

/// 百万 tokens → 显示串
fn fmt_tokens(millions: f64) -> String {
    if millions >= 1000.0 {
        format!("{:.1}B", millions / 1000.0)
    } else {
        format!("{:.0}M", millions)
    }
}

/// 趋势图悬浮卡: 整列分解 / 单色块详情
#[derive(Clone)]
enum TrendTip {
    /// x, y, 桶标签, 明细(名称, 颜色, 值), 桶总量
    Column(f64, f64, String, Vec<(String, &'static str, f64)>, f64),
    /// x, y, 桶标签, 模型名, 颜色, 值
    Segment(f64, f64, String, String, &'static str, f64),
}

/// 用量趋势大面板: 左侧累加直方图(按模型堆叠) + 右侧数据位。
/// 时间窗: 今天(24h)/本周(7d)/本月(30d)/今年(12mo)。
#[component]
fn TrendPanel(timeframe: Signal<&'static str>) -> Element {
    let tf = timeframe();
    let buckets = api::overview::fetch_trend(tf);
    let models = api::overview::fetch_models();
    let names: Vec<&str> = models.iter().map(|m| m.0).collect();

    let total_all: f64 = buckets.iter().map(|b| b.total).sum();
    let max_total = buckets
        .iter()
        .map(|b| b.total)
        .fold(0.0f64, f64::max)
        .max(1.0);
    let avg = total_all / buckets.len().max(1) as f64;
    let peak = buckets
        .iter()
        .max_by(|a, b| a.total.partial_cmp(&b.total).unwrap())
        .unwrap();

    let mut per_model_tot = vec![0.0f64; names.len()];
    for b in &buckets {
        for (i, v) in b.per_model.iter().enumerate() {
            per_model_tot[i] += v;
        }
    }
    let mut order: Vec<usize> = (0..names.len()).collect();
    order.sort_by(|&a, &b| per_model_tot[b].partial_cmp(&per_model_tot[a]).unwrap());
    order.truncate(3);

    // 共享悬浮卡: 整列模式列明细, 色块模式列单模型。fixed 定位不受滚动影响。
    let mut tip = use_signal(|| None::<TrendTip>);

    rsx! {
        section { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-5 transition-all duration-300 hover:border-zinc-700",
            div { class: "mb-4 flex flex-wrap items-center justify-between gap-3",
                div {
                    h2 { class: "text-sm font-medium text-zinc-300", "热门模型 · 用量趋势" }
                }
                div { class: "flex items-center gap-4",
                    div { class: "text-right",
                        p { class: "text-2xl font-semibold leading-none text-zinc-100", "{fmt_tokens(total_all)}" }
                        p { class: "mt-1 text-[11px] text-zinc-600", "令牌(合计)" }
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
            div { class: "grid grid-cols-1 gap-5 xl:grid-cols-3",
                // 左: 累加直方图(带横向虚网格 + 两级悬浮卡)
                div {
                    class: "xl:col-span-2",
                    onmouseleave: move |_| tip.set(None),
                    div { class: "relative",
                        div { class: "pointer-events-none absolute inset-0 flex flex-col justify-between py-0", aria_hidden: "true",
                            for frac in [1.0f64, 0.75, 0.5, 0.25] {
                                div { class: "relative w-full border-t border-dashed border-zinc-800",
                                    span { class: "absolute -top-2 right-0 text-[10px] text-zinc-600", "{fmt_tokens(max_total * frac)}" }
                                }
                            }
                            div { class: "relative w-full border-t border-dashed border-zinc-800",
                                span { class: "absolute -top-2 right-0 text-[10px] text-zinc-600", "0" }
                            }
                        }
                        div { class: "relative flex h-56 items-end", style: "gap: 3px",
                            for b in buckets.iter() {
                                {
                                    let hpct = (b.total / max_total * 100.0).max(3.0);
                                    let label = b.label.clone();
                                    // 列模式明细: 非零模型按量降序
                                    let mut col_rows: Vec<(String, &'static str, f64)> = b
                                        .per_model
                                        .iter()
                                        .enumerate()
                                        .filter(|(_, v)| **v > 0.01)
                                        .map(|(i, &v)| (names[i].to_string(), MODEL_COLORS[i % MODEL_COLORS.len()], v))
                                        .collect();
                                    col_rows.sort_by(|a, z| z.2.partial_cmp(&a.2).unwrap());
                                    let col_total = b.total;
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

                                            // 直方图有色堆叠容器
                                            div {
                                                class: "relative flex w-full flex-col-reverse overflow-hidden rounded-[3px] transition-all duration-200 gap-[1.5px]",
                                                style: "height: {hpct:.1}%",
                                                for (i, v) in b.per_model.iter().enumerate() {
                                                    {
                                                        let seg_label_1 = b.label.clone();
                                                        let seg_name_1 = names[i].to_string();
                                                        let seg_label_2 = b.label.clone();
                                                        let seg_name_2 = names[i].to_string();
                                                        let seg_color = MODEL_COLORS[i % MODEL_COLORS.len()];
                                                        let seg_val = *v;
                                                        rsx! {
                                                            div {
                                                                class: "pointer-events-auto w-full cursor-pointer hover:brightness-125 transition-all rounded-[1px]",
                                                                style: "height: {(v / b.total * 100.0):.1}%; background: {seg_color}",
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
                        for b in buckets.iter() {
                            span { class: "flex-1 truncate text-center",
                                if b.show_label { "{b.label}" }
                            }
                        }
                    }
                }
                // 右: 数据位 + Top3 图例 (标准深灰弱边框)
                div { class: "flex flex-col justify-between gap-5 rounded-xl border border-zinc-800 bg-zinc-900/50 p-5",
                    div { class: "grid grid-cols-2 gap-3",
                        div {
                            p { class: "text-[11px] text-zinc-600", "峰值桶" }
                            p { class: "mt-1 truncate text-sm font-semibold text-zinc-100", "{peak.label}" }
                            p { class: "text-xs font-mono text-zinc-500", "{fmt_tokens(peak.total)}" }
                        }
                        div {
                            p { class: "text-[11px] text-zinc-600", "平均每桶" }
                            p { class: "mt-1 text-sm font-semibold text-zinc-100", "{fmt_tokens(avg)}" }
                            p { class: "text-xs font-mono text-zinc-500", "均值" }
                        }
                        div {
                            p { class: "text-[11px] text-zinc-600", "活跃模型" }
                            p { class: "mt-1 text-sm font-semibold text-zinc-100", "{names.len()} 个" }
                            p { class: "text-xs font-mono text-zinc-500", "均有产出" }
                        }
                        div {
                            p { class: "text-[11px] text-zinc-600", "区间总量" }
                            p { class: "mt-1 text-sm font-semibold text-zinc-100", "{fmt_tokens(total_all)}" }
                            p { class: "text-xs font-mono text-zinc-500", "tokens" }
                        }
                    }
                    div { class: "border-t border-zinc-800/80 pt-3",
                        p { class: "mb-2 text-[11px] font-medium text-zinc-500", "主力模型 Top3" }
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
                                span { "总计 :" }
                                span { class: "font-mono font-semibold text-zinc-100", "{fmt_tokens(total)}" }
                            }
                            div { class: "flex flex-col gap-1.5",
                                for (name, color, v) in rows.iter() {
                                    div { class: "flex items-center justify-between gap-4 text-xs",
                                        div { class: "flex items-center min-w-0",
                                            span { class: "h-2 w-2 shrink-0 rounded-[2px]", style: "background: {color}" }
                                            span { class: "truncate text-zinc-300 ml-2", "{name}" }
                                        }
                                        span { class: "shrink-0 font-mono font-medium text-zinc-100", "{fmt_tokens(*v)}" }
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
                                span { class: "shrink-0 font-mono font-bold text-zinc-100", "{fmt_tokens(v)}" }
                            }
                        }
                    },
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
            class: "pointer-events-none fixed z-50 rounded-xl border border-zinc-700/80 bg-zinc-900/95 p-3 text-xs shadow-2xl backdrop-blur-md transition-all duration-150 ease-out {opacity_class} max-sm:left-3! max-sm:right-3! max-sm:bottom-4! max-sm:top-auto! max-sm:transform-none! max-sm:w-auto!",
            style: "left: {x}px; top: {clamped_y}px; transform: {transform}; max-width: calc(100vw - 24px);",
            p { class: "mb-1.5 text-xs font-semibold text-zinc-400", "{label}" }
            {children}
        }
    }
}
