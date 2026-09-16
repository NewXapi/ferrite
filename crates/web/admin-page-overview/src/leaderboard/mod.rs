//! 排行榜页 — 「模型实力榜（演示）」与「真实用量榜」双区块并存。
//!
//! - 模型实力榜(演示): 六维演示数据层 ([`data`]) + 立绘海报翻牌卡 ([`cards`]) + 汇总图表
//!   ([`charts`])。后端暂无价格、速度、上下文、成功率等维度端点,演示数值的出处与免责
//!   见 [`data`] 模块头声明,页面标题以「（演示）」字样标注,待真实源就绪后替换。
//! - 真实用量榜: 数据来自真实 `GET /api/log/top?by=model`(按模型聚合的消费日志),
//!   展示真实存在的三个口径 —— tokens / 调用数 / 费用(quota,$ 口径),指标名与轴标签
//!   如实反映口径;行内附增长率(tokens 环比,复用 api.rs 纯函数)与份额,另有
//!   上升/下跌最快名次变动双卡与厂商份额卡,见 [`insights`]。

mod cards;
mod charts;
pub mod data;
pub mod insights;

use dioxus::prelude::*;

use crate::api::{UsageTopRow, fmt_usd, growth_of, share_text, top_usage_api, window_start};
use cards::{MiniRadarCard, PosterImageCard};
use charts::{GroupQuotaCard, ModelDistributionCard, PerformanceLatencyCard};
use data::{MODELS, ModelStat, composite};
use insights::{
    MoversCards, MoversState, VendorShareCard, previous_window_start, top_usage_between,
};

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

/// 内部计费额度 → 美元展示串已复用 api.rs 的 `fmt_usd`(500000 = $1),
/// 与总览页 Top10 同口径;本文件不再保留 ¥ 折算的旧实现。

/// 排行榜取数口径:同一批 /api/log/top 聚合行,按不同字段重排展示。
/// 用枚举而非 fn 指针传参:component 宏会为 props 生成 PartialEq,函数指针比较不可靠。
#[derive(Clone, Copy, PartialEq)]
enum RankMetric {
    /// prompt + completion tokens 合计
    Tokens,
    /// 消费请求数
    Calls,
    /// 计费额度(500000 = $1,复用 api.rs fmt_usd)
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
            Self::Quota => fmt_usd(r.quota),
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
    // 份额分母 = 当榜行值合计(后端拉回的整批行,不截前 10);
    // 口径跟随所选 metric(Tokens/Calls/Quota 各自占各自口径的合计)
    let total: i64 = rows.iter().map(|r| metric.of(r)).sum();
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
            // 双列摊开(参照 new-api model-leaderboard 对半切,移动端单列)
            div { class: "grid grid-cols-1 gap-x-5 gap-y-2.5 pt-1 md:grid-cols-2",
                for (i, r) in top_n {
                    {
                        let v = metric.of(r);
                        let width_pct = (v as f64 / max_v as f64 * 100.0).max(2.0);
                        let value_text = metric.fmt(r);
                        // 增长率恒按 tokens 口径环比(与总览页 Top10 一致);
                        // 份额跟随所选 metric 口径
                        let growth = growth_of(r.previous_tokens, r.tokens);
                        let share = share_text(v, total);
                        rsx! {
                            div { key: "{r.name}", class: "flex items-center gap-2.5",
                                span { class: "flex h-5 w-5 shrink-0 items-center justify-center rounded bg-zinc-800/80 text-[10px] font-medium text-zinc-400 shadow-sm", "{i + 1}" }
                                div { class: "min-w-0 flex-1",
                                    div { class: "flex items-center justify-between gap-3",
                                        span { class: "truncate text-xs font-medium text-zinc-200", "{r.name}" }
                                        div { class: "flex shrink-0 flex-col items-end gap-0.5",
                                            span { class: "font-mono text-xs font-semibold tabular-nums text-zinc-100", "{value_text}" }
                                            div { class: "flex items-center gap-1.5 text-[10px] leading-none",
                                                if let Some(g) = growth {
                                                    span { class: "font-medium tabular-nums {g.text_class()}", "{g.label()}" }
                                                }
                                                if let Some(s) = share {
                                                    span { class: "tabular-nums text-zinc-500", "{s}" }
                                                }
                                            }
                                        }
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
            p { class: "border-t border-zinc-800/60 pt-2.5 text-[10px] text-zinc-500",
                "增长率为 tokens 环比(上一等长窗);份额为行值占当榜合计"
            }
        }
    }
}

/// 模型实力榜(演示)区块: 头牌翻牌卡 + 立绘海报卡阵列 + 汇总图表, 全部由 data 层演示数值推导。
/// 排序口径与恢复前版本一致: 按六维综合分降序。
#[component]
fn DemoBoard() -> Element {
    let mut ranked: Vec<&ModelStat> = MODELS.iter().collect();
    ranked.sort_by(|a, b| composite(b).partial_cmp(&composite(a)).unwrap());

    rsx! {
        section { "data-testid": "leaderboard-demo", role: "region", "aria-label": "模型实力榜（演示）",
            class: "flex flex-col gap-6 md:gap-8",
            div { class: "flex flex-wrap items-center justify-between gap-3 border-b border-zinc-800/80 pb-4",
                div {
                    h2 { class: "text-lg font-bold tracking-tight text-zinc-100 md:text-xl", "模型实力榜（演示）" }
                    p { class: "mt-1 text-xs text-zinc-400", "正面展示立绘与雷达图，点击卡牌可 3D 翻转查看六维综合评测与详细指标" }
                }
                span { class: "rounded-full border border-zinc-800 bg-zinc-900 px-3 py-1 text-xs text-zinc-400",
                    "共收录 {ranked.len()} 款主流模型"
                }
            }
            // 头牌翻牌卡: 综合分前五, 立绘交替斜角
            section { class: "grid grid-cols-1 gap-3 md:grid-cols-3 md:gap-4",
                for (i, m) in ranked.iter().take(5).copied().enumerate() {
                    MiniRadarCard {
                        rank: i + 1,
                        lean: if i % 2 == 0 { -4.0 } else { 0.0 },
                        model: m,
                    }
                }
            }
            // 海报翻牌卡大阵列
            section { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-5",
                for (i, m) in ranked.iter().copied().enumerate() {
                    PosterImageCard { rank: i + 1, model: m }
                }
            }
            // 底部数据分析图表 (参考 new-api / sub2api / wildtoken)
            section { class: "grid grid-cols-1 gap-4 xl:grid-cols-3 pt-2",
                ModelDistributionCard {}
                PerformanceLatencyCard {}
                GroupQuotaCard {}
            }
        }
    }
}

/// 模型用量排行榜: 真实 /api/log/top(by=model) 聚合 + 演示实力榜区块。
/// 时间窗与总览页同口径(今天=24h / 本周=7d / 本月=30d / 今年=365d)。
#[component]
pub fn LeaderboardPanel() -> Element {
    let mut timeframe = use_signal(|| "本月");
    let mut rows = use_signal(Vec::<UsageTopRow>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut reload = use_signal(|| 0u32);
    let mut movers = use_signal(|| MoversState::Loading);

    use_effect(move || {
        let tf = timeframe();
        let _ = reload();
        loading.set(true);
        err.set(None);
        movers.set(MoversState::Loading);
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
        // 上升/下跌最快:当前窗与上一等长窗各一次 by=model top20 聚合,
        // 名次变动在前端算(口径 tokens,与增长环比一致)。与主榜单独立
        // 拉取,失败只降级本区,不拖垮三张榜单
        spawn(async move {
            let cur_start = window_start(tf);
            let prev_start = previous_window_start(tf);
            let cur = top_usage_api("model", &cur_start, 20).await;
            let prev = top_usage_between("model", &prev_start, &cur_start, 20).await;
            movers.set(match (cur, prev) {
                (Ok(c), Ok(p)) => MoversState::Ready { cur: c, prev: p },
                (Err(e), _) | (_, Err(e)) => MoversState::Failed(e.to_string()),
            });
        });
    });

    let data = rows();
    let is_loading = loading();
    let error = err();
    let tf = timeframe();

    rsx! {
        div { class: "flex flex-col gap-6 p-4 md:gap-8 md:p-6",
            // 区块一: 模型实力榜(演示) — 恢复 #154 前的立绘卡牌阵列, 数值为演示数据
            DemoBoard {}

            // 区块一点五: 厂商份额(真实区上方,demo 之后、用量榜之前;复用 by=model 聚合)
            VendorShareCard { rows: data.clone(), loading: is_loading }

            // ===== 区块二: 真实用量榜(真实 /api/log/top 聚合, #154 接线原样保留) =====
            div { class: "flex flex-wrap items-center justify-between gap-3 border-b border-zinc-800/80 pb-4",
                div {
                    h2 { class: "text-lg font-bold tracking-tight text-zinc-100 md:text-xl", "真实用量榜" }
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
                            subtitle: "窗口内计费额度(500000 = $1)",
                            testid: "leaderboard-quota",
                            metric: RankMetric::Quota,
                            rows: data,
                        }
                    }
                    // 上升/下跌最快双卡(真实区新增,follows 当前 timeframe)
                    MoversCards { state: movers() }
                }
            }
        }
    }
}
