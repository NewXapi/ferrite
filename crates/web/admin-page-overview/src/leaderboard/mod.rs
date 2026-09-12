//! 排行榜页 — 数据来自真实 `GET /api/log/top?by=model`(按模型聚合的消费日志)。
//!
//! 旧的六维雷达/立绘翻牌卡为纯 mock 形态(data.rs / cards.rs / charts.rs 已移除):
//! 后端没有价格、速度、上下文、成功率等维度,本页降级为「列表 + 条形」展示
//! 真实存在的三个口径 —— tokens / 调用数 / 费用(quota),指标名与轴标签如实反映口径。

use dioxus::prelude::*;

use crate::api::{UsageTopRow, top_usage_api, window_start};

/// 模型配色(内联 hex, 不走 Tailwind 扫描) — 沿用旧 charts.rs 的色板。
const MODEL_COLORS: [&str; 10] = [
    "#3b82f6", "#c4b5fd", "#a78bfa", "#facc15", "#fb8500", "#34d399", "#22d3ee", "#f472b6",
    "#a3e635", "#a1a1aa",
];

/// 原始 tokens → 显示串 (K/M/B),与 overview.rs 的口径一致(该文件不在本 PR 白名单内,此处局部实现)。
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

/// 内部计费额度 → 人民币展示 (500000 = ¥1),与 overview.rs 的口径一致。
fn fmt_cny(quota: i64) -> String {
    format!("¥{:.2}", quota as f64 / 500_000.0)
}

/// 排行榜取数口径:同一批 /api/log/top 聚合行,按不同字段重排展示。
/// 用枚举而非 fn 指针传参:component 宏会为 props 生成 PartialEq,函数指针比较不可靠。
#[derive(Clone, Copy, PartialEq)]
enum RankMetric {
    /// prompt + completion tokens 合计
    Tokens,
    /// 消费请求数
    Calls,
    /// 计费额度(500000 = ¥1)
    Quota,
}

impl RankMetric {
    /// 该口径下行的排序键。
    fn of(self, r: &UsageTopRow) -> i64 {
        match self {
            Self::Tokens => r.tokens,
            Self::Calls => r.calls,
            Self::Quota => r.quota,
        }
    }

    /// 该口径下行的展示串。
    fn fmt(self, r: &UsageTopRow) -> String {
        match self {
            Self::Tokens => fmt_raw(r.tokens),
            Self::Calls => r.calls.to_string(),
            Self::Quota => fmt_cny(r.quota),
        }
    }
}

/// 单个排行卡:按 `metric` 从真实聚合行里取前 N,画名次 + 名称 + 条形 + 数值。
#[component]
fn RankCard(
    title: &'static str,
    subtitle: &'static str,
    testid: &'static str,
    metric: RankMetric,
    rows: Vec<UsageTopRow>,
) -> Element {
    // 排序副本:后端只保证 tokens 降序,calls/quota 榜需本地重排
    let mut sorted: Vec<&UsageTopRow> = rows.iter().collect();
    sorted.sort_by_key(|r| std::cmp::Reverse(metric.of(r)));
    let max_v = sorted.first().map(|r| metric.of(r)).unwrap_or(0).max(1);
    let top_n: Vec<(usize, &UsageTopRow)> = sorted
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, r)| (i, *r))
        .collect();

    rsx! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-5 space-y-4",
            "data-testid": "{testid}",
            div {
                h3 { class: "text-sm font-semibold text-zinc-100", "{title}" }
                p { class: "text-[11px] text-zinc-500", "{subtitle}" }
            }
            div { class: "space-y-2.5 pt-1",
                for (i, r) in top_n {
                    {
                        let v = metric.of(r);
                        let width_pct = (v as f64 / max_v as f64 * 100.0).max(2.0);
                        let value_text = metric.fmt(r);
                        rsx! {
                            div { key: "{r.name}", class: "flex items-center gap-2.5",
                                span { class: "flex h-5 w-5 shrink-0 items-center justify-center rounded bg-zinc-800/80 text-[10px] font-medium text-zinc-400 shadow-sm", "{i + 1}" }
                                div { class: "min-w-0 flex-1",
                                    div { class: "flex items-center justify-between gap-3",
                                        span { class: "truncate text-xs font-medium text-zinc-200", "{r.name}" }
                                        span { class: "shrink-0 font-mono text-xs font-semibold tabular-nums text-zinc-100", "{value_text}" }
                                    }
                                    div { class: "mt-1 h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                                        div {
                                            class: "h-full rounded-full transition-all duration-300",
                                            style: "width: {width_pct:.1}%; background: {MODEL_COLORS[i % MODEL_COLORS.len()]}",
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

/// 模型用量排行榜: 真实 /api/log/top(by=model) 聚合。
/// 时间窗与总览页同口径(今天=24h / 本周=7d / 本月=30d / 今年=365d)。
#[component]
pub fn LeaderboardPanel() -> Element {
    let mut timeframe = use_signal(|| "本月");
    let mut rows = use_signal(Vec::<UsageTopRow>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut reload = use_signal(|| 0u32);

    use_effect(move || {
        let tf = timeframe();
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let start = window_start(tf);
            match top_usage_api("model", &start, 10).await {
                Ok(items) => {
                    rows.set(items);
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    let data = rows();
    let is_loading = loading();
    let error = err();
    let tf = timeframe();

    rsx! {
        div { class: "flex flex-col gap-6 p-4 md:gap-8 md:p-6",
            div { class: "flex flex-wrap items-center justify-between gap-3 border-b border-zinc-800/80 pb-4",
                div {
                    h2 { class: "text-lg font-bold tracking-tight text-zinc-100 md:text-xl", "模型用量排行榜" }
                    p { class: "mt-1 text-xs text-zinc-400", "按后端消费日志聚合(/api/log/top):窗口内各模型的 Token 消耗、调用次数与费用" }
                }
                span { class: "rounded-full border border-zinc-800 bg-zinc-900 px-3 py-1 text-xs text-zinc-400",
                    "窗口内 {data.len()} 个模型有调用"
                }
            }

            // 时间窗切换 + 刷新(口径与总览页 trend 一致)
            div { class: "flex flex-wrap items-center justify-between gap-3",
                div { class: "flex items-center gap-1.5 rounded-lg border border-zinc-800 bg-zinc-950 p-1",
                    for t in ["今天", "本周", "本月", "今年"] {
                        button {
                            key: "{t}",
                            "data-testid": "leaderboard-timeframe-{t}",
                            class: "rounded-md px-2.5 py-1 text-xs font-medium transition-colors",
                            class: if tf == t { "bg-zinc-800 text-zinc-100 shadow-sm" } else { "text-zinc-400 hover:text-zinc-200" },
                            onclick: move |_| timeframe.set(t),
                            "{t}"
                        }
                    }
                }
                button {
                    class: "rounded-xl border border-zinc-700 px-3 py-2 text-xs text-zinc-300 transition-colors hover:bg-zinc-800",
                    "data-testid": "refresh-leaderboard",
                    onclick: move |_| reload.set(reload() + 1),
                    "刷新"
                }
            }

            section { "data-testid": "leaderboard-usage", role: "region", "aria-label": "模型用量排行",
                if let Some(e) = error {
                    div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 px-4 py-6 text-center",
                        p { class: "text-sm text-red-300", "加载排行数据失败" }
                        p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                        button {
                            class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                            "data-testid": "retry-leaderboard",
                            onclick: move |_| reload.set(reload() + 1),
                            "重试"
                        }
                    }
                } else if is_loading {
                    div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                        p { class: "text-zinc-400", "正在加载排行数据…" }
                    }
                } else if data.is_empty() {
                    div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                        p { class: "text-zinc-400", "该时间窗内暂无调用数据" }
                        p { class: "mt-1 text-xs text-zinc-600", "发起一次 /v1 调用后这里会展示真实用量排行" }
                    }
                } else {
                    div { class: "grid grid-cols-1 gap-4 xl:grid-cols-3 pt-2",
                        RankCard {
                            title: "Token 消耗 Top",
                            subtitle: "窗口内 prompt + completion tokens 合计",
                            testid: "leaderboard-tokens",
                            metric: RankMetric::Tokens,
                            rows: data.clone(),
                        }
                        RankCard {
                            title: "调用次数 Top",
                            subtitle: "窗口内消费请求数",
                            testid: "leaderboard-calls",
                            metric: RankMetric::Calls,
                            rows: data.clone(),
                        }
                        RankCard {
                            title: "费用消耗 Top",
                            subtitle: "窗口内计费额度(500000 = ¥1)",
                            testid: "leaderboard-quota",
                            metric: RankMetric::Quota,
                            rows: data,
                        }
                    }
                }
            }
        }
    }
}
