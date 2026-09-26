//! 渠道健康度面板 — 数据来自真实 `/api/monitor`（monitor_history 探活聚合）
//! 与 `/api/channel`（key → 名称映射）。
//!
//! 探活未产生任何记录时显示诚实空态,不再使用 mock 假数据。

use dioxus::prelude::*;

use super::shared::{
    CHANNEL_FALLBACK, DASH, HEALTH_AVAIL, HEALTH_EMPTY, HEALTH_EMPTY_HINT, HEALTH_LATENCY,
    HEALTH_LOADING, HEALTH_MONITORED, HEALTH_PROBES, NEUTRAL_NO_DATA, SEC_HEALTH,
};
use crate::api;
use client::ApiClient;
use ui::card::{Card, CardContent, CardHeader, CardTitle};

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

    // 进面板自动拉一次(use_effect 无信号依赖 → 仅挂载执行);失败时 err 只留
    // 在内存,不驱动任何 UI 重试(8090 预览反馈①②)。
    use_effect(move || {
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
                            .unwrap_or_else(|| {
                                format!("{CHANNEL_FALLBACK} {}", &key[..8.min(key.len())])
                            })
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
                    // 按可用率升序(最差的排前面,运维视角);NaN 防御:SQL 聚合理论
                    // 上不会产出 NaN,但 unwrap_or(Equal) 免除 panic 面
                    out.sort_by(|a, b| {
                        let av = |r: &ChannelHealthRow| r.availability.unwrap_or(0.0);
                        av(a)
                            .partial_cmp(&av(b))
                            .unwrap_or(std::cmp::Ordering::Equal)
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
        (n.to_string(), HEALTH_MONITORED),
        (
            avg_avail
                .map(|v| format!("{:.1}%", v * 100.0))
                .unwrap_or_else(|| DASH.into()),
            HEALTH_AVAIL,
        ),
        (total_probes.to_string(), HEALTH_PROBES),
        (
            avg_latency
                .map(|v| format!("{:.0}ms", v))
                .unwrap_or_else(|| DASH.into()),
            HEALTH_LATENCY,
        ),
    ];

    rsx! {
        Card {
            hoverable: true,
            CardHeader {
                CardTitle { class: "{ui::T_text_lg} text-foreground", "{SEC_HEALTH}" }
            }
            CardContent {
                section { "data-testid": "channel-health",
                    class: "space-y-3",
                    // 拉取失败 → 中性占位(8090 预览反馈①):与空态同风格的
                    // dashed 骨架 + 一行 muted 小字;失败文案 / HTTP 状态码 /
                    // 重试按钮均不上 UI。重拉时机:本面板随 tab 卸载/重挂(use_effect 重新
                    // 执行即重新拉取),时间窗切换亦触发;err 仅留在内存不渲染。
                    if err.is_some() {
                        div { class: "rounded-2xl border border-dashed border-border bg-card/50 py-10 text-center",
                            p { class: "{ui::T_text_sm} {ui::T_text_zinc_500}", "{NEUTRAL_NO_DATA}" }
                        }
                    } else if loading {
                        div { class: "rounded-2xl border border-dashed border-border bg-card/50 py-10 text-center",
                            p { class: "text-muted-foreground", "{HEALTH_LOADING}" }
                        }
                    } else if n == 0 {
                        div { class: "rounded-2xl border border-dashed border-border bg-card/50 py-10 text-center",
                            p { class: "text-muted-foreground", "{HEALTH_EMPTY}" }
                            p { class: "mt-1 {ui::T_text_xs} text-muted-foreground/70", "{HEALTH_EMPTY_HINT}" }
                        }
                    } else {
                        // 汇总卡
                        div { class: "grid grid-cols-2 gap-3 md:grid-cols-4",
                            for (value, label) in summary {
                                div { class: "rounded-xl border border-border bg-card px-4 py-3 transition-[border-color] duration-150 hover:border-secondary-hover cursor-default",
                                    p { class: "{ui::T_text_base} {ui::T_font_semibold} text-foreground", "{value}" }
                                    p { class: "mt-0.5 {ui::T_text_xs} text-muted-foreground", "{label}" }
                                }
                            }
                        }
                        // 逐渠道可用率
                        div { class: "rounded-xl border border-border bg-card/60 p-4 space-y-3 transition-[border-color] duration-150 hover:border-secondary-hover",
                            for r in data {
                                div { class: "flex items-center gap-3",
                                    span { class: "w-28 shrink-0 truncate {ui::T_text_sm} text-foreground", "{r.name}" }
                                    div { class: "h-2 flex-1 overflow-hidden rounded-full bg-muted",
                                        div {
                                            class: "h-full rounded-full transition-all",
                                            style: "width: {(r.availability.unwrap_or(0.0) * 100.0):.1}%; background: {health_color(r.availability)}",
                                        }
                                    }
                                    span { class: "w-14 shrink-0 text-right {ui::T_text_xs} font-mono text-muted-foreground",
                                        {r.availability.map(|v| format!("{:.1}%", v * 100.0)).unwrap_or_else(|| DASH.into())}
                                    }
                                    span { class: "w-24 shrink-0 text-right {ui::T_text_xs} font-mono text-muted-foreground/70",
                                        "{r.ok_count}/{r.total}"
                                    }
                                    span { class: "w-16 shrink-0 text-right {ui::T_text_xs} font-mono text-muted-foreground/70",
                                        {r.avg_latency_ms.map(|v| format!("{:.0}ms", v)).unwrap_or_else(|| DASH.into())}
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

/// 可用率 → 状态色：≥99% 绿,≥95% 黄,否则红。
fn health_color(availability: Option<f64>) -> &'static str {
    match availability {
        Some(v) if v >= 0.99 => "#34d399",
        Some(v) if v >= 0.95 => "#facc15",
        Some(_) => "#f87171",
        None => "#52525b",
    }
}
