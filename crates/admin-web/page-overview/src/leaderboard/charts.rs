//! 汇总图表: 模型用量与成本分布 / 网关耗时与 SLA 矩阵 / 分组配额与倍率分布.
//! 参考 new-api / sub2api / wildtoken 数据面板设计.

use dioxus::prelude::*;

use crate::api::leaderboard::MODELS;

const MODEL_COLORS: [&str; 10] = [
    "#3b82f6", "#c4b5fd", "#a78bfa", "#facc15", "#fb8500",
    "#34d399", "#22d3ee", "#f472b6", "#a3e635", "#a1a1aa",
];

/// 模型用量与成本占比分布 (参考 new-api consumption-distribution & sub2api ModelDistribution)
#[component]
pub fn ModelDistributionCard() -> Element {
    let mut timeframe = use_signal(|| "本月");
    let total_tokens: f64 = MODELS.iter().map(|m| m.tokens).sum();

    rsx! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-5 space-y-4",
            div { class: "flex items-center justify-between gap-3",
                div {
                    h3 { class: "text-sm font-semibold text-zinc-100", "模型用量与成本占比" }
                    p { class: "text-[11px] text-zinc-500", "Token 消耗分布与费用占比" }
                }
                div { class: "flex items-center rounded-lg border border-zinc-800 bg-zinc-950 p-0.5",
                    for tf in ["今天", "本周", "本月", "至今"] {
                        button {
                            key: "{tf}",
                            class: "rounded px-2 py-0.5 text-[11px] transition-colors",
                            class: if timeframe() == tf { "bg-zinc-800 font-medium text-zinc-100 shadow-sm" } else { "text-zinc-500 hover:text-zinc-300" },
                            onclick: move |_| timeframe.set(tf),
                            "{tf}"
                        }
                    }
                }
            }

            // 比例分段条 (类似 GitHub 语言占比条)
            div { class: "flex h-2.5 w-full overflow-hidden rounded-full bg-zinc-800/80 gap-[1.5px]",
                for (i, m) in MODELS.iter().take(6).enumerate() {
                    {
                        let pct = (m.tokens / total_tokens * 100.0).max(2.0);
                        let color = MODEL_COLORS[i % MODEL_COLORS.len()];
                        rsx! {
                            div {
                                class: "h-full transition-all duration-300",
                                style: "width: {pct:.1}%; background: {color};",
                                title: "{m.name}: {pct:.1}%"
                            }
                        }
                    }
                }
            }

            // 模型用量列表
            div { class: "space-y-2.5 pt-1",
                for (i, m) in MODELS.iter().take(5).enumerate() {
                    {
                        let pct = m.tokens / total_tokens * 100.0;
                        let color = MODEL_COLORS[i % MODEL_COLORS.len()];
                        let cost_est = m.tokens * m.price / 1000.0;
                        rsx! {
                            div {
                                key: "{m.name}",
                                class: "group flex items-center justify-between rounded-lg p-1.5 transition-colors hover:bg-zinc-800/50",
                                div { class: "flex items-center gap-2.5 min-w-0",
                                    span { class: "h-2.5 w-2.5 shrink-0 rounded-[2px]", style: "background: {color}" }
                                    div { class: "min-w-0",
                                        p { class: "truncate text-xs font-medium text-zinc-200 group-hover:text-white", "{m.name}" }
                                        p { class: "text-[10px] text-zinc-500", "{m.daily_req / 1e3:.0}K 次请求 · 上下文 {m.ctx:.0}K" }
                                    }
                                }
                                div { class: "text-right shrink-0 pl-2",
                                    p { class: "font-mono text-xs font-semibold tabular-nums text-zinc-100", "{m.tokens / 1e3:.1}B" }
                                    p { class: "font-mono text-[10px] text-zinc-500", "${cost_est:.2} ({pct:.1}%)" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 网关响应与 SLA 性能矩阵 (参考 new-api performance-overview & wildtoken latency)
#[component]
pub fn PerformanceLatencyCard() -> Element {
    rsx! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-5 space-y-4",
            div { class: "flex items-center justify-between",
                div {
                    h3 { class: "text-sm font-semibold text-zinc-100", "网关响应与 SLA 性能矩阵" }
                    p { class: "text-[11px] text-zinc-500", "端到端 P50 延迟、吞吐与高可用" }
                }
                div { class: "flex items-center gap-1.5 rounded-full border border-emerald-500/20 bg-emerald-500/10 px-2.5 py-0.5 text-[11px] font-medium text-emerald-400",
                    span { class: "h-1.5 w-1.5 rounded-full bg-emerald-400 animate-pulse" }
                    "SLA 99.94%"
                }
            }

            // 4 项关键健康指标
            div { class: "grid grid-cols-2 gap-2.5",
                div { class: "rounded-lg border border-zinc-800/80 bg-zinc-950/60 p-2.5",
                    p { class: "text-[10px] text-zinc-500", "P50 均值延迟" }
                    p { class: "mt-0.5 font-mono text-base font-bold text-zinc-100", "820 ms" }
                }
                div { class: "rounded-lg border border-zinc-800/80 bg-zinc-950/60 p-2.5",
                    p { class: "text-[10px] text-zinc-500", "P90 尾部延迟" }
                    p { class: "mt-0.5 font-mono text-base font-bold text-zinc-100", "1,450 ms" }
                }
                div { class: "rounded-lg border border-zinc-800/80 bg-zinc-950/60 p-2.5",
                    p { class: "text-[10px] text-zinc-500", "峰值吞吐 TPS" }
                    p { class: "mt-0.5 font-mono text-base font-bold text-zinc-100", "4,210 tok/s" }
                }
                div { class: "rounded-lg border border-zinc-800/80 bg-zinc-950/60 p-2.5",
                    p { class: "text-[10px] text-zinc-500", "平均成功率" }
                    p { class: "mt-0.5 font-mono text-base font-bold text-emerald-400", "99.86%" }
                }
            }

            // 模型延迟对比条
            div { class: "space-y-2 pt-1",
                for m in MODELS.iter().take(5) {
                    {
                        let width_pct = (m.p50 / 3.0 * 100.0).min(100.0);
                        rsx! {
                            div { key: "{m.name}", class: "space-y-1 text-xs",
                                div { class: "flex items-center justify-between text-zinc-300",
                                    span { class: "font-medium", "{m.name}" }
                                    div { class: "flex items-center gap-3 font-mono text-[11px] text-zinc-400",
                                        span { "{m.speed:.0} tok/s" }
                                        span { class: "text-zinc-100 font-semibold", "{m.p50:.1}s" }
                                    }
                                }
                                div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                                    div {
                                        class: if m.p50 < 1.0 { "h-full rounded-full bg-emerald-400" } else if m.p50 < 2.0 { "h-full rounded-full bg-blue-400" } else { "h-full rounded-full bg-amber-400" },
                                        style: "width: {width_pct:.1}%"
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

/// 分组配额与倍率分布 (参考 sub2api GroupDistribution)
#[component]
pub fn GroupQuotaCard() -> Element {
    let groups = [
        ("default", 1.0f64, "4.2B", 85, "#3b82f6"),
        ("vip", 1.5f64, "2.8B", 60, "#a78bfa"),
        ("svip", 2.0f64, "1.9B", 40, "#facc15"),
        ("internal", 0.5f64, "820M", 20, "#34d399"),
    ];

    rsx! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-5 space-y-4",
            div { class: "flex items-center justify-between",
                div {
                    h3 { class: "text-sm font-semibold text-zinc-100", "分组配额与倍率分布" }
                    p { class: "text-[11px] text-zinc-500", "租户路由分组及倍率消耗" }
                }
                span { class: "rounded border border-zinc-800 bg-zinc-950 px-2 py-0.5 text-[10px] text-zinc-400",
                    "4 个活跃分组"
                }
            }

            div { class: "space-y-3 pt-1",
                for (name, mult, quota, pct, color) in groups {
                    div { key: "{name}", class: "space-y-1.5",
                        div { class: "flex items-center justify-between text-xs",
                            div { class: "flex items-center gap-2",
                                span { class: "h-2 w-2 rounded-full", style: "background: {color}" }
                                span { class: "font-medium text-zinc-200 uppercase", "{name}" }
                                span { class: "rounded bg-zinc-800/80 px-1.5 py-0.2 text-[10px] text-zinc-400 font-mono", "{mult:.1}x" }
                            }
                            span { class: "font-mono font-semibold text-zinc-100", "{quota}" }
                        }
                        div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                            div { class: "h-full rounded-full transition-all duration-300", style: "width: {pct}%; background: {color}" }
                        }
                    }
                }
            }

            div { class: "mt-4 rounded-lg border border-zinc-800/80 bg-zinc-950/60 p-3 text-xs text-zinc-400 space-y-1",
                div { class: "flex justify-between", span { "默认路由权重:" } span { class: "text-zinc-200 font-mono font-medium", "Priority 优先" } }
                div { class: "flex justify-between", span { "自动降级熔断:" } span { class: "text-emerald-400 font-mono font-medium", "已开启" } }
            }
        }
    }
}
