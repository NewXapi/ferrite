//! 汇总图表: 综合排行 / 延迟山脊图 / 性价比气泡图.
//! 全部从 data::MODELS 派生, 不含独立状态.

use dioxus::prelude::*;

use crate::api::{composite, ModelStat, MODELS};

/// 综合排行: 按六维均值排序.
#[component]
pub fn RankListCard() -> Element {
    let mut ranked: Vec<(&ModelStat, f64)> = MODELS.iter().map(|m| (m, composite(m))).collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    rsx! {
        div { class: "self-start rounded-xl border border-zinc-800 bg-zinc-900 p-5 xl:col-span-2",
            h2 { class: "mb-4 text-sm font-medium text-zinc-300", "综合排行" }
            div { class: "space-y-3",
                for (i, (m, score)) in ranked.iter().enumerate() {
                    {
                        let num_cls = if i == 0 { "text-zinc-100" } else { "text-zinc-500" };
                        let bar_cls = if i == 0 { "bg-zinc-200" } else { "bg-zinc-500" };
                        rsx! {
                        div { class: "grid grid-cols-[auto_1fr_auto] items-center gap-3",
                            span { class: "w-5 text-right text-sm font-semibold {num_cls}", "{i + 1}" }
                            span { class: "truncate text-sm text-zinc-300", "{m.name}" }
                            span { class: "text-sm text-zinc-400", "{score:.1}" }
                            div { class: "col-span-2 col-start-2 h-1.5 overflow-hidden rounded-full bg-zinc-800",
                                div {
                                    class: "h-full rounded-full {bar_cls}",
                                    style: "width: {score:.1}%",
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

/// 延迟山脊图: 每模型一行的重叠分布曲线.
#[component]
pub fn RidgeCard() -> Element {
    const W: f64 = 340.0;
    const ROW: f64 = 24.0;
    const N: usize = 40;

    let rows: Vec<(String, String, f64)> = MODELS
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let baseline = 30.0 + i as f64 * ROW;
            let mu = 8.0 + (i * 13 % 20) as f64;
            let sigma = 3.0 + (i % 3) as f64 * 1.5;
            let mut d = format!("M 56 {baseline:.1}");
            for x in 0..=N {
                let x = x as f64;
                let y = 60.0 * (-(x - mu).powi(2) / (2.0 * sigma * sigma)).exp()
                    + 22.0 * (-(x - mu - 9.0).powi(2) / (2.0 * 5.0 * 5.0)).exp();
                d.push_str(&format!(" L {:.1} {:.1}", 56.0 + x * 7.0, baseline - y * 0.28));
            }
            d.push_str(&format!(" L {:.1} {baseline:.1} Z", 56.0 + N as f64 * 7.0));
            (m.name.to_string(), d, baseline)
        })
        .collect();
    let height = 30.0 + MODELS.len() as f64 * ROW + 6.0;

    rsx! {
        div { class: "self-start rounded-xl border border-zinc-800 bg-zinc-900 p-5 md:col-span-2 xl:col-span-3",
            h2 { class: "mb-4 text-sm font-medium text-zinc-300", "延迟分布 · 山脊图" }
            svg { class: "w-full overflow-visible", view_box: "-62 -6 {W + 64.0} {height + 10.0}",
                for (name, d, baseline) in &rows {
                    path {
                        d: "{d}",
                        fill: "rgba(161,161,170,0.10)",
                        stroke: "#d4d4d8", stroke_width: "0.8",
                    }
                    text {
                        x: "50", y: "{baseline - 2.0:.1}",
                        text_anchor: "end",
                        class: "fill-zinc-500 text-[8.5px]",
                        "{name}"
                    }
                }
            }
            div { class: "mt-1 flex justify-between pl-14 text-xs text-zinc-600",
                span { "快" }
                span { "P50 延迟 (相对)" }
                span { "慢" }
            }
        }
    }
}

/// 性价比气泡图: x = 价格 (sqrt), y = 速度, r = 用量份额.
#[component]
pub fn BubbleCard() -> Element {
    const PLOT_W: f64 = 280.0;
    const PLOT_H: f64 = 176.0;
    let to_x = |price: f64| 44.0 + price.sqrt() / 15f64.sqrt() * PLOT_W;
    let to_y = |speed: f64| 16.0 + (130.0 - speed) / 80.0 * PLOT_H;
    let max_req = MODELS.iter().map(|m| m.daily_req).fold(0.0, f64::max);

    rsx! {
        div { class: "self-start rounded-xl border border-zinc-800 bg-zinc-900 p-5 xl:col-span-2",
            h2 { class: "mb-4 text-sm font-medium text-zinc-300", "性价比定位 · 气泡图" }
            svg { class: "w-full overflow-visible", view_box: "-4 -4 352 232",
                line { x1: "44", y1: "16", x2: "44", y2: "192", stroke: "#27272a" }
                line { x1: "44", y1: "192", x2: "336", y2: "192", stroke: "#27272a" }
                text { x: "44", y: "212", text_anchor: "start", class: "fill-zinc-600 text-[9px]", "低价" }
                text { x: "336", y: "212", text_anchor: "end", class: "fill-zinc-600 text-[9px]", "高价 ($/1M)" }
                text { x: "8", y: "20", text_anchor: "start", class: "fill-zinc-600 text-[9px]", "tok/s" }
                for m in MODELS {
                    {
                        let bx = to_x(m.price);
                        let by = to_y(m.speed);
                        let r = 4.0 + (m.daily_req / max_req).sqrt() * 9.0;
                        rsx! {
                            circle {
                                cx: "{bx:.1}", cy: "{by:.1}", r: "{r:.1}",
                                fill: "rgba(161,161,170,0.18)",
                                stroke: "#a1a1aa", stroke_width: "0.8",
                            }
                            text {
                                x: "{bx:.1}", y: "{by + r + 10.0:.1}",
                                text_anchor: "middle",
                                class: "fill-zinc-500 text-[8.5px]",
                                "{m.name}"
                            }
                        }
                    }
                }
            }
        }
    }
}
