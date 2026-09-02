use chrono::{Datelike, Duration, Local, NaiveDate};
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
            
            // Top-level stats
            section { class: "grid grid-cols-2 gap-3 md:grid-cols-4 xl:grid-cols-5",
                for &(value, label) in stats.iter() {
                    StatCard { value, label }
                }
            }
            
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

            // 面板区: 同一套栏数, 热力图按时间范围占 3/2/1 栏
            section { class: "grid grid-cols-1 gap-3 md:grid-cols-3 md:gap-4 xl:grid-cols-5",
            ActivityGrid {}
            // old Breakdowns removed in favor of Top-10 lists
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
    "#3b82f6", "#c4b5fd", "#a78bfa", "#facc15", "#fb8500",
    "#34d399", "#22d3ee", "#f472b6", "#a3e635", "#a1a1aa",
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
    let max_total = buckets.iter().map(|b| b.total).fold(0.0f64, f64::max).max(1.0);
    let avg = total_all / buckets.len().max(1) as f64;
    let peak = buckets.iter().max_by(|a, b| a.total.partial_cmp(&b.total).unwrap()).unwrap();

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
                    p { class: "mt-0.5 text-xs text-zinc-600",
                        match tf {
                            "今天" => "过去24小时内各模型的逐小时 Token 用量",
                            "本周" => "最近7天内各模型的每日 Token 用量",
                            "本月" => "过去一个月内各模型的每日 Token 用量",
                            _ => "今年各模型逐月 Token 用量",
                        }
                    }
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
                                            // 整列鼠标控制
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
                                                    // Only update position for Column tip, ignore if a Segment tip is active
                                                    if matches!(*tip.read(), Some(TrendTip::Column(..))) {
                                                        let p = evt.data.client_coordinates();
                                                        tip.set(Some(TrendTip::Column(p.x, p.y, c_label.clone(), c_rows.clone(), col_total)));
                                                    }
                                                }
                                            },
                                            onmouseleave: move |_| tip.set(None),
                                            
                                            // 悬停整列: 通顶全高半透明背景光柱
                                            div { class: "pointer-events-none absolute inset-x-0 top-0 bottom-0 rounded-sm transition-colors duration-150 group-hover:bg-zinc-100/10" }
                                            
                                            // 直方图容器
                                            div {
                                                class: "pointer-events-none relative flex w-full flex-col-reverse overflow-hidden rounded-[3px] transition-all duration-200 gap-[1.5px]",
                                                style: "height: {hpct:.1}%",
                                                for (i, v) in b.per_model.iter().enumerate() {
                                                    {
                                                        let seg_label_1 = b.label.clone();
                                                        let seg_name_1 = names[i].to_string();
                                                        let seg_label_2 = b.label.clone();
                                                        let seg_name_2 = names[i].to_string();
                                                        let seg_color = MODEL_COLORS[i % MODEL_COLORS.len()];
                                                        let seg_val = *v;
                                                        let col_label_restore = label.clone();
                                                        let col_rows_restore = col_rows.clone();
                                                        
                                                        rsx! {
                                                            div {
                                                                class: "pointer-events-auto w-full cursor-pointer hover:brightness-125 transition-all rounded-[1px]",
                                                                style: "height: {(v / b.total * 100.0):.1}%; background: {seg_color}",
                                                                // 精确单色块模式: 悬停时只显示该单一模型
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
                                                                onmouseleave: move |evt| {
                                                                    evt.stop_propagation();
                                                                    let p = evt.data.client_coordinates();
                                                                    tip.set(Some(TrendTip::Column(p.x, p.y, col_label_restore.clone(), col_rows_restore.clone(), col_total)));
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
                            div { class: "mb-3 flex items-center justify-between border-b border-zinc-800 pb-2 text-xs text-zinc-400",
                                span { "总计 :" }
                                span { class: "font-mono font-semibold text-zinc-100", "{fmt_tokens(total)}" }
                            }
                            div { class: "flex flex-col gap-2.5",
                                for (name, color, v) in rows.iter() {
                                    div { class: "flex items-center justify-between gap-6 text-xs",
                                        div { class: "flex items-center gap-3 min-w-0",
                                            span { class: "h-2 w-2 shrink-0 rounded-[2px]", style: "background: {color}" }
                                            span { class: "truncate text-zinc-300", "{name}" }
                                        }
                                        span { class: "shrink-0 font-mono font-medium text-zinc-100", "{fmt_tokens(*v)}" }
                                    }
                                }
                            }
                        }
                    },
                    TrendTip::Segment(x, y, label, name, color, v) => rsx! {
                        TrendTooltipContainer { x, y, label,
                            div { class: "flex items-center justify-between gap-8 py-1 text-xs",
                                div { class: "flex items-center gap-3 min-w-0",
                                    span { class: "h-2.5 w-2.5 shrink-0 rounded-[2px]", style: "background: {color}" }
                                    span { class: "text-zinc-300 font-medium", "{name}" }
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
    // 屏幕宽度自适应：在窄屏/右侧边缘时自动向左翻转，避免被裁剪或溢出
    let transform = if x > 400.0 {
        "translate(calc(-100% - 20px), -50%)"
    } else {
        "translate(20px, -50%)"
    };
    let opacity_class = if x == 0.0 && y == 0.0 { "opacity-0 scale-95" } else { "opacity-100 scale-100" };
    rsx! {
        div {
            class: "pointer-events-none fixed z-50 min-w-[220px] whitespace-nowrap rounded-xl bg-zinc-900/95 p-4 text-xs shadow-2xl backdrop-blur-md transition-all duration-200 ease-out {opacity_class}",
            style: "left: {x}px; top: {y}px; transform: {transform}",
            p { class: "mb-3 text-sm font-semibold text-zinc-100", "{label}" }
            {children}
        }
    }
}

struct DayCell {
    in_range: bool,
    level: u8,
    date: String,
    tokens: String,
    cost: String,
}

/// Token activity heatmap: GitHub-style calendar, responsive (no scrollbar).
/// Cells are square via aspect-square, week columns flex-1 to fill the panel.
/// Range tabs (全年 / 半年 / 近3月) sit centered under the month labels; the
/// panel's grid span shrinks with the range (xl: 3栏 → 2栏 → 1栏).
#[component]
fn ActivityGrid() -> Element {

    let mut range = use_signal(|| 0u8);

    let today = Local::now().date_naive();
    // 全年 364d / 半年 182d / 近3月 91d, aligned back to Sunday for full weeks.
    let days = match range() {
        0 => 364,
        1 => 182,
        _ => 91,
    };
    let first = today - Duration::days(days);
    let aligned = first - Duration::days(first.weekday().num_days_from_sunday() as i64);
    let n_weeks = ((today - aligned).num_days() / 7 + 1) as usize;

    // 5栏网格里的占位: 3个月是最小单位 (1栏), 半年 2栏, 全年 3栏。
    let span = match range() {
        0 => "md:col-span-2 xl:col-span-3",
        1 => "md:col-span-2 xl:col-span-2",
        _ => "xl:col-span-1",
    };

    let weeks: Vec<Vec<DayCell>> = (0..n_weeks)
        .map(|w| {
            (0..7)
                .map(|r| {
                    let day = aligned + Duration::days((w * 7 + r) as i64);
                    let in_range = day >= first && day <= today;
                    let (level, tokens, cost) = day_mock(day);
                    DayCell {
                        in_range,
                        level,
                        date: day.format("%b %-d, %Y").to_string(),
                        tokens,
                        cost,
                    }
                })
                .collect()
        })
        .collect();

    // Month labels: the week containing the 1st of a month carries its label;
    // positions are percentages so they follow the flex-stretched columns.
    let month_labels: Vec<(usize, String)> = weeks
        .iter()
        .enumerate()
        .filter_map(|(w, _col)| {
            let day = aligned + Duration::days((w * 7) as i64);
            (0..7)
                .map(|r| day + Duration::days(r as i64))
                .find(|d| d.day() == 1)
                .map(|d| (w, format!("{}\u{6708}", d.month())))
        })
        .collect();

    // One shared tooltip; fixed positioning is immune to any scroll offset.
    // ponytail: single tooltip + full re-render per hover, fine at mock scale.
    let mut tip = use_signal(|| None::<(f64, f64, String, String, String)>);

    let ranges = [
        (0u8, "\u{5168}\u{5E74}"),
        (1u8, "\u{534A}\u{5E74}"),
        (2u8, "\u{8FD1}3\u{4E2A}\u{6708}"),
    ];

    rsx! {
        section { class: "self-start {span} rounded-xl border border-zinc-800 bg-zinc-900 p-5",
            h2 { class: "mb-4 text-sm font-medium text-zinc-300", "Token \u{6D3B}\u{52A8}" }
            div { class: "relative",
                div {
                    div { class: "flex", style: "gap: 3px",
                        for week in weeks {
                            div { class: "flex flex-1 flex-col", style: "gap: 3px",
                                for day in week {
                                    if day.in_range {
                                        {
                                            let date = day.date.clone();
                                            let tokens = day.tokens.clone();
                                            let cost = day.cost.clone();
                                            rsx! {
                                                div {
                                                    class: "relative aspect-square w-full cursor-pointer rounded-[2px] transition-all duration-300 hover:ring-2 hover:ring-zinc-400 hover:scale-[1.2] hover:z-20 {heat_shade(day.level)}",
                                                    onmouseenter: move |evt| {
                                                        let p = evt.data.client_coordinates();
                                                        tip.set(Some((p.x, p.y, date.clone(), tokens.clone(), cost.clone())));
                                                    },
                                                    onmouseleave: move |_| tip.set(None),
                                                }
                                            }
                                        }
                                    } else {
                                        div { class: "aspect-square w-full opacity-0" }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "relative mt-2 h-4 text-xs text-zinc-600",
                        for (w, label) in month_labels {
                            span {
                                class: "absolute",
                                style: "left: {(w as f64 / n_weeks as f64) * 100.0:.2}%",
                                "{label}"
                            }
                        }
                    }
                }
                if let Some((x, y, date, tokens, cost)) = tip() {
                    div {
                        class: "pointer-events-none fixed z-50 whitespace-nowrap rounded-lg border border-zinc-700 bg-zinc-950/95 px-3 py-2 shadow-xl",
                        style: "left: {x}px; top: {y - 10.0}px; transform: translate(-50%, -100%)",
                        p { class: "text-xs font-semibold text-zinc-100", "{date}" }
                        p { class: "mt-1 flex justify-between gap-4 text-xs text-zinc-400",
                            span { "Tokens" }
                            span { class: "text-zinc-300", "{tokens}" }
                        }
                        p { class: "flex justify-between gap-4 text-xs text-zinc-400",
                            span { "Cost" }
                            span { class: "text-zinc-300", "{cost}" }
                        }
                    }
                }
            }
            // Range switcher: pill-style segmented dots (横向版), centered.
            div { class: "mt-4 flex justify-center",
                div { class: "flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-950 px-3 py-2",
                    for (id, name) in ranges {
                        button {
                            class: if range() == id {
                                "h-2 w-5 rounded-full bg-zinc-100 transition-all"
                            } else {
                                "h-2 w-2 rounded-full bg-zinc-700 transition-all hover:bg-zinc-500"
                            },
                            "aria-label": "{name}",
                            title: "{name}",
                            onclick: move |_| range.set(id),
                        }
                    }
                }
            }
        }
    }
}
/// Deterministic per-day mock: activity level 0..=4 plus display values.
fn day_mock(d: NaiveDate) -> (u8, String, String) {
    let s = d.num_days_from_ce() as u64;
    let mut h = s.wrapping_mul(0x517cc1b727220a95);
    h ^= h >> 32;
    let r = (h % 100) as u8;
    let level = match r {
        0..=24 => 0,
        25..=54 => 1,
        55..=79 => 2,
        80..=93 => 3,
        _ => 4,
    };
    let tokens = match level {
        0 => "0",
        1 => "12.4k",
        2 => "86.1k",
        3 => "342.8k",
        _ => "1.24m",
    };
    let cost = match level {
        0 => "$0.00",
        1 => "$0.02",
        2 => "$0.14",
        3 => "$0.58",
        _ => "$2.10",
    };
    (level, tokens.to_string(), cost.to_string())
}

/// Tailwind background class per activity level (0..=4).
fn heat_shade(level: u8) -> &'static str {
    match level {
        0 => "bg-zinc-800/40",
        1 => "bg-zinc-700",
        2 => "bg-zinc-500",
        3 => "bg-zinc-300",
        _ => "bg-zinc-100",
    }
}
