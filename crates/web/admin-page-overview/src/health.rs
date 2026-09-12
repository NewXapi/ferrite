//! 渠道健康度面板 — 数据来自真实 `/api/monitor`（monitor_history 探活聚合）
//! 与 `/api/channel`（key → 名称映射）。
//!
//! 探活未产生任何记录时显示诚实空态,不再使用 mock 假数据。

use dioxus::prelude::*;

use crate::api;
use client::ApiClient;

/// 一行渠道健康汇总。
#[derive(Clone, PartialEq)]
struct ChannelHealthRow {
    name: String,
    availability: Option<f64>,
    total: i64,
    ok_count: i64,
    avg_latency_ms: Option<f64>,
}

/// 渠道健康度面板：汇总卡 + 逐渠道可用率条。
#[component]
pub fn ChannelHealth() -> Element {
    let mut rows = use_signal(Vec::<ChannelHealthRow>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut reload = use_signal(|| 0u32);

    use_effect(move || {
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            // monitor 只回 channel_key,名称从渠道列表映射
            let (mon_r, ch_r) = (
                api::monitor_api(7).await,
                api::list_channels_api(&client).await,
            );
            match (mon_r, ch_r) {
                (Ok(items), Ok(channels)) => {
                    let name_of = |key: &str| -> String {
                        channels
                            .iter()
                            .find(|c| c.key == key)
                            .map(|c| c.name.clone())
                            .unwrap_or_else(|| format!("渠道 {}", &key[..8.min(key.len())]))
                    };
                    let mut out: Vec<ChannelHealthRow> = items
                        .into_iter()
                        .map(|a| ChannelHealthRow {
                            name: name_of(&a.channel_key),
                            availability: a.availability,
                            total: a.total,
                            ok_count: a.ok_count,
                            avg_latency_ms: a.avg_latency_ms,
                        })
                        .collect();
                    // 按可用率升序(最差的排前面,运维视角)
                    out.sort_by(|a, b| {
                        let av = |r: &ChannelHealthRow| r.availability.unwrap_or(0.0);
                        av(a).partial_cmp(&av(b)).unwrap()
                    });
                    rows.set(out);
                    loading.set(false);
                }
                (Err(e), _) | (_, Err(e)) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    let data = rows();
    let loading = loading();
    let err = err();

    // 汇总：渠道数 / 平均可用率 / 探活总数 / 平均延迟
    let n = data.len();
    let avg_avail = if n > 0 {
        let sum: f64 = data.iter().filter_map(|r| r.availability).sum();
        let with_avail = data.iter().filter(|r| r.availability.is_some()).count();
        if with_avail > 0 {
            Some(sum / with_avail as f64)
        } else {
            None
        }
    } else {
        None
    };
    let total_probes: i64 = data.iter().map(|r| r.total).sum();
    let avg_latency = {
        let lats: Vec<f64> = data.iter().filter_map(|r| r.avg_latency_ms).collect();
        if lats.is_empty() {
            None
        } else {
            Some(lats.iter().sum::<f64>() / lats.len() as f64)
        }
    };

    let summary: [(String, &str); 4] = [
        (n.to_string(), "受监控渠道"),
        (
            avg_avail
                .map(|v| format!("{:.1}%", v * 100.0))
                .unwrap_or_else(|| "—".into()),
            "平均可用率(7天)",
        ),
        (total_probes.to_string(), "探活总数(7天)"),
        (
            avg_latency
                .map(|v| format!("{:.0}ms", v))
                .unwrap_or_else(|| "—".into()),
            "平均延迟",
        ),
    ];

    rsx! {
        div { class: "space-y-3",
            div { class: "flex items-center justify-between",
                h2 { class: "text-lg font-medium text-zinc-100", "渠道健康 (近 7 天)" }
                button {
                    class: "shrink-0 rounded-xl border border-zinc-700 px-3 py-2 text-xs text-zinc-300 transition-colors hover:bg-zinc-800",
                    "data-testid": "refresh-health",
                    onclick: move |_| reload.set(reload() + 1),
                    "刷新"
                }
            }
            section { "data-testid": "channel-health",
                class: "space-y-3",
                if let Some(e) = err {
                    div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 px-4 py-6 text-center",
                        p { class: "text-sm text-red-300", "加载渠道健康失败" }
                        p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                        button {
                            class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                            onclick: move |_| reload.set(reload() + 1),
                            "重试"
                        }
                    }
                } else if loading {
                    div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-10 text-center",
                        p { class: "text-zinc-400", "正在加载渠道健康…" }
                    }
                } else if n == 0 {
                    div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-10 text-center",
                        p { class: "text-zinc-400", "暂无探活数据" }
                        p { class: "mt-1 text-xs text-zinc-600", "monitor_history 为空 —— 渠道探活开始产生记录后这里会展示真实可用率" }
                    }
                } else {
                    // 汇总卡
                    div { class: "grid grid-cols-2 gap-3 md:grid-cols-4",
                        for (value, label) in summary {
                            div { class: "rounded-xl border border-zinc-800 bg-zinc-900 px-4 py-3 transition-all duration-200 hover:border-zinc-700 hover:bg-zinc-900/80 hover:-translate-y-0.5 hover:shadow-md hover:shadow-black/20 cursor-default",
                                p { class: "text-base font-semibold text-zinc-100", "{value}" }
                                p { class: "mt-0.5 text-xs text-zinc-500", "{label}" }
                            }
                        }
                    }
                    // 逐渠道可用率
                    div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 space-y-3 transition-all duration-300 hover:border-zinc-700 hover:shadow-lg hover:shadow-black/20",
                        for r in data {
                            div { class: "flex items-center gap-3",
                                span { class: "w-28 shrink-0 truncate text-sm text-zinc-300", "{r.name}" }
                                div { class: "h-2 flex-1 overflow-hidden rounded-full bg-zinc-800",
                                    div {
                                        class: "h-full rounded-full transition-all",
                                        style: "width: {(r.availability.unwrap_or(0.0) * 100.0):.1}%; background: {health_color(r.availability)}",
                                    }
                                }
                                span { class: "w-14 shrink-0 text-right text-xs font-mono text-zinc-400",
                                    {r.availability.map(|v| format!("{:.1}%", v * 100.0)).unwrap_or_else(|| "—".into())}
                                }
                                span { class: "w-24 shrink-0 text-right text-xs font-mono text-zinc-600",
                                    "{r.ok_count}/{r.total}"
                                }
                                span { class: "w-16 shrink-0 text-right text-xs font-mono text-zinc-600",
                                    {r.avg_latency_ms.map(|v| format!("{:.0}ms", v)).unwrap_or_else(|| "—".into())}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 可用率 → 状态色：≥99% 绿,≥95% 黄,否则红。
fn health_color(availability: Option<f64>) -> &'static str {
    match availability {
        Some(v) if v >= 0.99 => "#34d399",
        Some(v) if v >= 0.95 => "#facc15",
        Some(_) => "#f87171",
        None => "#52525b",
    }
}
