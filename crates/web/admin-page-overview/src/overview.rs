use dioxus::prelude::*;

use client::ApiClient;
use contract::api::usage::DashboardSummaryDto;
use ui::components::card::{Card, CardContent, CardHeader};

use crate::api::{self, TrendBucketFE};

// Layout convention (共享给所有面板组件, 详见仓库 README.md):
//   页面网格  `grid-cols-1 md:grid-cols-3 lg:grid-cols-5`  —— 手机 1 栏 / 平板 3 栏 / Web 5 栏。
//   小卡片(统计卡)占 1 栏; 宽面板并排: 热力图类 `md:col-span-2 xl:col-span-3`,
//   列表/分布类 `md:col-span-1 xl:col-span-2`; 手机端一律堆叠, 定宽内容用横向滚动。

/// Overview canvas (总览): responsive odd-column grid — 2 cols on mobile,
/// 3 on md, 5 on xl; wide sections span every column.
/// 数据全部来自真实后端：/api/dashboard + /api/log/trend + /api/log/top。
#[component]
pub fn OverviewPanel() -> Element {
    // ponytail: full UI overhaul to add breakdown cards for all timeframes in one go.
    let timeframe = use_signal(|| "今天"); // "今天", "本周", "本月", "今年"

    // 实时汇总:挂载时 use_effect 拉 GET /api/dashboard,写入 summary signal。
    let mut summary = use_signal(|| None::<DashboardSummaryDto>);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut reload = use_signal(|| 0u32);

    // 趋势 + Top10 共用的窗口数据源：timeframe/reload 变化即重拉。
    let mut top_users = use_signal(Vec::<TopRowFE>::new);
    let mut top_models = use_signal(Vec::<TopRowFE>::new);
    let mut top_users_total = use_signal(|| 0i64);
    let mut top_models_total = use_signal(|| 0i64);
    // 统计卡 sparkline（今日 24h → 12 点）：额度卡用 tokens 序列、请求卡用 calls 序列。
    let mut spark_tokens = use_signal(Vec::<f64>::new);
    let mut spark_calls = use_signal(Vec::<f64>::new);
    let mut buckets = use_signal(Vec::<TrendBucketFE>::new);
    let mut model_order = use_signal(Vec::<String>::new);
    let mut data_loading = use_signal(|| true);
    let mut data_err = use_signal(|| None::<String>);

    use_effect(move || {
        let tf = timeframe();
        let _ = reload();
        data_loading.set(true);
        data_err.set(None);
        spawn(async move {
            let start = api::window_start(tf);
            let granularity = match tf {
                "今天" => "hour",
                "今年" => "month",
                _ => "day",
            };
            // sparkline 固定取「今天」24h 窗：与两张今日卡的口径一致，且与当前
            // 切换的 timeframe 解耦；start 只算一次、拉数与重切共用，避免跨小时漂移。
            let spark_start = api::window_start("今天");
            // 趋势 + 两个 Top 榜 + sparkline 源并行拉；主数据一个失败即报错（数据完整性优先）。
            let (trend_r, users_r, models_r, spark_r) = (
                api::trend_api(granularity, &start).await,
                api::top_usage_api("user", &start, 10).await,
                api::top_usage_api("model", &start, 10).await,
                api::trend_api("hour", &spark_start).await,
            );
            match (trend_r, users_r, models_r) {
                (Ok(rows), Ok(users), Ok(models)) => {
                    let (b, order) = api::pivot_trend(rows, tf);
                    buckets.set(b);
                    model_order.set(order);
                    // 用户榜按消耗折 $ 展示、模型榜按 tokens 展示；份额 = 行值占前 10
                    // 名合计；增长率 = 本窗 tokens 环比上一等长窗口 previous_tokens。
                    let u_tot: i64 = users.iter().map(|r| r.quota).sum();
                    top_users_total.set(u_tot);
                    top_users.set(
                        users
                            .iter()
                            .map(|r| TopRowFE {
                                name: r.name.clone(),
                                amount: api::fmt_usd(r.quota),
                                share: api::share_text(r.quota, u_tot),
                                growth: api::growth_of(r.previous_tokens, r.tokens),
                            })
                            .collect(),
                    );
                    let m_tot: i64 = models.iter().map(|r| r.tokens).sum();
                    top_models_total.set(m_tot);
                    top_models.set(
                        models
                            .iter()
                            .map(|r| TopRowFE {
                                name: r.name.clone(),
                                amount: fmt_raw(r.tokens),
                                share: api::share_text(r.tokens, m_tot),
                                growth: api::growth_of(r.previous_tokens, r.tokens),
                            })
                            .collect(),
                    );
                    // sparkline 是装饰性最佳努力：拉失败保持空 → 卡内渲染等高占位
                    // （诚实降级），不影响主数据展示。
                    if let Ok(srows) = spark_r {
                        let (tokens, calls) = api::hourly_sums(&srows, &spark_start);
                        spark_tokens.set(api::reslice_sum(&tokens, 12));
                        spark_calls.set(api::reslice_sum(&calls, 12));
                    }
                    data_loading.set(false);
                }
                (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
                    data_err.set(Some(e.to_string()));
                    data_loading.set(false);
                }
            }
        });
    });

    use_effect(move || {
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            match api::get_dashboard_summary_api(&client).await {
                Ok(d) => {
                    summary.set(Some(d));
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    // 把实时 DTO 展开成 (值, 中文标签) 卡片列表(rsx! 之外计算,避免宏内 let)。
    let stats_opt: Option<Vec<(String, &'static str)>> = summary().as_ref().map(dashboard_stats);
    // 统计卡四元组:值 / 标签 / sparkline 序列(仅今日两卡) / SVG 渐变 id。
    // 在 rsx! 之外整形——宏体内 let 不支持任意绑定,for 循环的元组解构才支持。
    // 仅「今日请求 / 今日额度」两张卡带 12 点迷你面积线;
    // 空序列(拉数失败或窗口无数据)由 Sparkline 渲染等高占位。
    let stat_cards: Vec<(String, &'static str, Option<Vec<f64>>, &'static str)> = stats_opt
        .iter()
        .flatten()
        .map(|(value, label)| {
            let (spark, gid) = match *label {
                "今日请求" => (Some(spark_calls()), "sparkline-requests"),
                "今日额度" => (Some(spark_tokens()), "sparkline-quota"),
                _ => (None, ""),
            };
            (value.clone(), *label, spark, gid)
        })
        .collect();
    // 数据新鲜度: asOf 的本地时间直接亮在统计卡区头部;解析失败不显示(诚实降级)。
    let as_of_time = summary()
        .as_ref()
        .and_then(|d| api::as_of_local_time(&d.as_of));
    let trend_loading = data_loading();
    let trend_err = data_err();
    // 用计算后的 buckets 判空 (单一数据源): pivot 后如果每桶 total 都是 0,
    // 则以渲染为准 — 与 setter 里的行判零语义一致, 但不会错位。
    let empty_window =
        !trend_loading && trend_err.is_none() && buckets().iter().all(|b| b.total <= 0.0);

    rsx! {
        div { class: "flex flex-col gap-3 p-4 md:gap-4 md:p-6",
            // 用量趋势大面板(含时间窗切换) —— 真实 /api/log/trend 聚合
            TrendPanel { timeframe, buckets, model_order, loading: trend_loading, err: trend_err, empty_window }

            // 渠道健康度(真实 /api/monitor 探活聚合)
            crate::health::ChannelHealth {}

            // 实时汇总统计卡(数据来自真实后端 /api/dashboard)
            div { class: "space-y-3",
                div { class: "flex items-center justify-between",
                    h2 { class: "text-lg font-medium text-foreground", "总览统计" }
                    div { class: "flex items-center gap-3",
                        // asOf 本地时间裸值(维护者要求:不写「数据截至」字样)
                        if let Some(t) = as_of_time {
                            span {
                                class: "text-xs font-mono tabular-nums text-muted-foreground",
                                "data-testid": "overview-as-of",
                                "{t}"
                            }
                        }
                        button {
                            class: "shrink-0 rounded-xl border border-border px-3 py-2 text-xs text-foreground/80 transition-colors hover:bg-accent",
                            "data-testid": "refresh-overview",
                            onclick: move |_| reload.set(reload() + 1),
                            "刷新"
                        }
                    }
                }
                section { "data-testid": "overview-stats",
                    class: "grid grid-cols-1 gap-3 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-5",
                    if let Some(e) = err() {
                        div { class: "col-span-full rounded-2xl border border-red-800/60 bg-red-950/40 px-4 py-6 text-center",
                            p { class: "text-sm text-red-300", "加载统计失败" }
                            p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                            button {
                                class: "mt-3 rounded-xl border border-border px-3 py-1.5 text-xs text-foreground/80 hover:bg-accent",
                                onclick: move |_| reload.set(reload() + 1),
                                "重试"
                            }
                        }
                    } else if loading() {
                        div { class: "col-span-full rounded-2xl border border-dashed border-border bg-card/50 py-10 text-center",
                            p { class: "text-muted-foreground", "正在加载统计…" }
                        }
                    } else if stats_opt.is_some() {
                        for (value, label, sparkline, gradient_id) in stat_cards {
                            StatCard { value, label, sparkline, gradient_id }
                        }
                        // 第 7 张卡:额度余量 + runway 可用天数(W1 后端已供 quotaRemaining)
                        if let Some(d) = summary() {
                            QuotaRemainingCard { remaining: d.quota_remaining, today: d.quota_today }
                        }
                    }
                }
            }

            // Top 10 breakdowns —— 真实 /api/log/top 聚合
            // 面板卡挂 hoverable（维护者要求悬停边框变亮的动态全站回归）;
            // 调用方 py-0!/gap-0!/px-4!/py-3!/p-4!/pb-3! 覆盖 Card 基串的
            // py-6/gap-6/px-6, 尾缀 ! 确保压过 Tailwind 同属性工具类, 保留原紧凑条头布局。
            section { class: "grid grid-cols-1 gap-3 md:grid-cols-2 lg:gap-4",
                // Top 10 Models
                Card {
                    hoverable: true,
                    class: "gap-0! overflow-hidden py-0!",
                    CardHeader {
                        class: "border-b border-border/50 px-4! py-3!",
                        div { class: "flex items-center justify-between gap-3",
                            h3 { class: "text-sm font-medium text-foreground", "消耗前十模型" }
                            div { class: "text-right", "data-testid": "top-models-total",
                                p { class: "text-sm font-semibold font-mono tabular-nums text-foreground", "{fmt_raw(top_models_total())}" }
                                p { class: "text-[10px] text-muted-foreground", "tokens 合计" }
                            }
                        }
                    }
                    CardContent {
                        class: "flex-1 space-y-3 p-4! pb-3!",
                        if top_models().is_empty() {
                            p { class: "py-6 text-center text-xs text-muted-foreground", "该时间窗内暂无调用" }
                        }
                        for (i, row) in top_models().iter().enumerate() {
                            TopRowItem { index: i, name: row.name.clone(), amount: row.amount.clone(), growth: row.growth.clone(), share: row.share.clone() }
                        }
                        p { class: "pt-1 text-[10px] leading-4 text-muted-foreground/60",
                            "份额为该行占前 10 名合计的比例 · 增长环比上一等长窗口 tokens"
                        }
                    }
                }

                // Top 10 Users
                Card {
                    hoverable: true,
                    class: "gap-0! overflow-hidden py-0!",
                    CardHeader {
                        class: "border-b border-border/50 px-4! py-3!",
                        div { class: "flex items-center justify-between gap-3",
                            h3 { class: "text-sm font-medium text-foreground", "消耗前十用户" }
                            div { class: "text-right", "data-testid": "top-users-total",
                                p { class: "text-sm font-semibold font-mono tabular-nums text-foreground", "{api::fmt_usd(top_users_total())}" }
                                p { class: "text-[10px] text-muted-foreground", "$ 合计" }
                            }
                        }
                    }
                    CardContent {
                        class: "flex-1 space-y-3 p-4! pb-3!",
                        if top_users().is_empty() {
                            p { class: "py-6 text-center text-xs text-muted-foreground", "该时间窗内暂无调用" }
                        }
                        for (i, row) in top_users().iter().enumerate() {
                            TopRowItem { index: i, name: row.name.clone(), amount: row.amount.clone(), growth: row.growth.clone(), share: row.share.clone() }
                        }
                        p { class: "pt-1 text-[10px] leading-4 text-muted-foreground/60",
                            "份额为该行占前 10 名合计的比例 · 增长环比上一等长窗口 tokens"
                        }
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
fn StatCard(
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

/// 统计卡底部的 12 点手绘迷你面积线（无图表库；配方见 deepdive §二-3：
/// 160x36 viewBox + preserveAspectRatio="none"、min-max 归一化、line+area 双 path、
/// area 用 currentColor 线性渐变 0.24→0、non-scaling-stroke 保证任意拉伸线宽恒定）。
///
/// 数据不足两点（空序列 / 单点）→ 渲染等高占位防布局跳动（诚实降级）。
#[component]
fn Sparkline(series: Vec<f64>, gradient_id: &'static str) -> Element {
    let Some((line_d, area_d)) = api::sparkline_svg_paths(&series, 160.0, 36.0) else {
        return rsx! {
            div {
                class: "pointer-events-none mt-2 h-9 rounded-md border border-dashed border-border/60 bg-accent/30",
                aria_hidden: "true",
            }
        };
    };
    rsx! {
        svg {
            class: "pointer-events-none mt-2 h-9 w-full text-muted-foreground",
            view_box: "0 0 160 36",
            preserve_aspect_ratio: "none",
            "aria-hidden": "true",
            "data-testid": "{gradient_id}",
            defs {
                linearGradient {
                    id: "{gradient_id}",
                    x1: "0",
                    y1: "0",
                    x2: "0",
                    y2: "1",
                    stop { offset: "0", stop_color: "currentColor", stop_opacity: "0.24" }
                    stop { offset: "1", stop_color: "currentColor", stop_opacity: "0" }
                }
            }
            path { d: "{area_d}", fill: "url(#{gradient_id})" }
            path {
                d: "{line_d}",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2.25",
                vector_effect: "non-scaling-stroke",
                stroke_linecap: "round",
                stroke_linejoin: "round",
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
fn QuotaRemainingCard(remaining: i64, today: i64) -> Element {
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

/// Top 榜单行渲染所需的前端视图（在 use_effect 内从 `UsageTopRow` 整形一次）。
#[derive(Clone, PartialEq)]
struct TopRowFE {
    name: String,
    /// 展示值：用户榜 `fmt_usd(quota)`、模型榜 `fmt_raw(tokens)`。
    amount: String,
    /// 行值占前 10 名合计的份额（合计 ≤ 0 时为 `None`，不显示）。
    share: Option<String>,
    /// 本窗 tokens 环比上一等长窗口的增长率（本窗为 0 时为 `None`，不显示）。
    growth: Option<api::Growth>,
}

/// Top 榜单行：名次 + 名称 + 右侧「数值在上、增长率与份额在下」双行列对齐。
#[component]
fn TopRowItem(
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

/// 把实时 `DashboardSummaryDto` 展开成 (值, 中文标签) 卡片列表,供总览统计区渲染。
///
/// 顺序与标签:`总用户=users`、`启用渠道=channels_enabled`、`令牌=tokens`、
/// `分组=groups`、`今日请求=requests_today`、`今日额度=quota_today`。
/// 今日额度按 `fmt_usd` 折算 $ 展示(500_000 = $1),与 Top10 用户榜同口径。
fn dashboard_stats(d: &DashboardSummaryDto) -> Vec<(String, &'static str)> {
    vec![
        (d.users.to_string(), "总用户"),
        (d.channels_enabled.to_string(), "启用渠道"),
        (d.tokens.to_string(), "令牌"),
        (d.groups.to_string(), "分组"),
        (d.requests_today.to_string(), "今日请求"),
        (api::fmt_usd(d.quota_today), "今日额度"),
    ]
}

/// 模型配色(内联 hex, 不走 Tailwind 扫描, 避免 @source 漏扫隐形)
const MODEL_COLORS: [&str; 10] = [
    "#3b82f6", "#c4b5fd", "#a78bfa", "#facc15", "#fb8500", "#34d399", "#22d3ee", "#f472b6",
    "#a3e635", "#a1a1aa",
];

/// 原始 tokens → 显示串 (K/M/B)
fn fmt_raw(n: i64) -> String {
    let v = n as f64;
    if v.abs() >= 1_000_000_000.0 {
        format!("{:.1}B", v / 1_000_000_000.0)
    } else if v.abs() >= 1_000_000.0 {
        format!("{:.1}M", v / 1_000_000.0)
    } else if v.abs() >= 1_000.0 {
        format!("{:.1}K", v / 1_000.0)
    } else {
        n.to_string()
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
/// 时间窗: 今天(24h 逐时)/本周(7d 逐天)/本月(30d 逐天)/今年(12mo 逐月)。
/// 数据来自真实 /api/log/trend 聚合;窗口内无调用时显示诚实空态。
#[component]
fn TrendPanel(
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
                                        // 列模式明细: 非零模型按量降序
                                        let mut col_rows: Vec<(String, &'static str, f64)> = b
                                            .per_model
                                            .iter()
                                            .enumerate()
                                            .filter(|(_, v)| **v > 0.01)
                                            .map(|(i, &v)| (names[i].clone(), MODEL_COLORS[i % MODEL_COLORS.len()], v))
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
                                    span { "总计 :" }
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
