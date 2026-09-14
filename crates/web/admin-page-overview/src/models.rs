//! 模型页 — 双区块并存,与排行榜页同构(演示区块在前、真实区块在后)。
//!
//! - 区块一「模型」: #154 删除前的三内部 tab 模型展示卡(概览 / 分组价格 / 待定),
//!   按 2db77d1^ 原样恢复,数据源为 admin-mock crate 的 `mock::models::MODELS`
//!   (纯演示数值,非后端数据),页头「mock · N 个」如实标注演示属性。
//! - 区块二「真实模型」: #154 接线的真实模型列表原样保留,数据来自真实
//!   `GET /api/models`(见 [`crate::api::list_models_api`]),后端没有的字段
//!   (价格、趋势、分组报价)不展示,不造数据;loading / error / empty 三态诚实,
//!   写法与 health.rs / overview.rs 一致。

use dioxus::prelude::*;

use client::ApiClient;
use mock::models::{MODELS, ModelInfo};

use crate::api::{self, ModelCardView};

/// Tab switcher glyph.
#[component]
fn TabGlyph(kind: u8) -> Element {
    rsx! {
        svg { class: "h-3.5 w-3.5", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2",
            match kind {
                // 概览
                0 => rsx! { circle { cx: "12", cy: "12", r: "8" } circle { cx: "12", cy: "12", r: "2.5", fill: "currentColor" } },
                // 分组
                1 => rsx! {
                    rect { x: "4", y: "4", width: "7", height: "7", rx: "1.5" }
                    rect { x: "13", y: "4", width: "7", height: "7", rx: "1.5" }
                    rect { x: "4", y: "13", width: "7", height: "7", rx: "1.5" }
                    rect { x: "13", y: "13", width: "7", height: "7", rx: "1.5" }
                },
                // 待定
                _ => rsx! {
                    circle { cx: "5", cy: "12", r: "1.5", fill: "currentColor" }
                    circle { cx: "12", cy: "12", r: "1.5", fill: "currentColor" }
                    circle { cx: "19", cy: "12", r: "1.5", fill: "currentColor" }
                },
            }
        }
    }
}

/// One model = one card. Three internal tabs: 概览 / 分组价格 / 待定.
/// Width and flow come from the parent layout; the card is self-contained.
#[component]
pub fn ModelCard(model: ModelInfo) -> Element {
    let mut tab = use_signal(|| 0u8);

    // Sparkline geometry (viewBox 200x56)
    let n = model.trend.len().max(2) as f32;
    let pts: Vec<(f32, f32)> = model
        .trend
        .iter()
        .enumerate()
        .map(|(i, v)| (i as f32 * (200.0 / (n - 1.0)), 50.0 - *v as f32 * 0.42))
        .collect();
    let line: String = pts
        .iter()
        .map(|(x, y)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ");
    let area = format!("0,56 {line} 200,56");
    let (last_x, last_y) = *pts.last().unwrap();
    let gid = format!("fill-{}", model.name.replace(['.', '-'], "_"));

    let card_cls = "flex flex-col gap-3 rounded-2xl border border-white/10 \
                    bg-gradient-to-b from-zinc-800/60 to-zinc-900/40 p-4 \
                    shadow-xl shadow-black/40 ring-1 ring-white/5 backdrop-blur-xl";

    rsx! {
        section { class: "{card_cls}",
            // Top bar: name + internal tab switcher
            header { class: "flex items-start justify-between gap-3",
                div { class: "min-w-0",
                    h3 { class: "truncate text-base font-semibold tracking-tight text-zinc-50", "{model.name}" }
                    p { class: "mt-0.5 text-xs text-zinc-500", "{model.vendor}" }
                }
                div { class: "flex shrink-0 gap-1.5",
                    for i in 0..3u8 {
                        button {
                            class: if tab() == i {
                                "flex h-7 w-7 items-center justify-center rounded-lg border border-white/10 bg-white/10 text-zinc-100 shadow-inner"
                            } else {
                                "flex h-7 w-7 items-center justify-center rounded-lg border border-white/5 bg-black/20 text-zinc-600 transition-colors hover:text-zinc-300"
                            },
                            onclick: move |_| tab.set(i),
                            TabGlyph { kind: i }
                        }
                    }
                }
            }

            // ---- tab 1: 概览 ----
            if tab() == 0 {
                // Price line
                div { class: "flex flex-wrap items-baseline gap-x-4 gap-y-1 text-sm",
                    span { class: "text-zinc-500", "输入 " b { class: "font-semibold tabular-nums text-zinc-100", "{model.price_input}" } }
                    span { class: "text-zinc-500", "输出 " b { class: "font-semibold tabular-nums text-zinc-100", "{model.price_output}" } }
                    span { class: "text-zinc-500", "缓存 " b { class: "font-semibold tabular-nums text-zinc-100", "{model.price_cache}" } }
                }
                p { class: "text-xs text-zinc-500", "{model.description}" }

                div { class: "border-t border-white/5" }

                // 数据展示
                div {
                    p { class: "text-[11px] uppercase tracking-wider text-zinc-600", "24h Tokens" }
                    p { class: "mt-1 text-2xl font-semibold tabular-nums tracking-tight text-zinc-50", "{model.tokens_24h}" }
                    p { class: "mt-0.5 text-xs tabular-nums text-zinc-500", "${model.cost_24h}" }
                    div { class: "mt-3 grid grid-cols-3 gap-2",
                        MiniStat { label: "请求", value: model.requests_24h }
                        MiniStat { label: "成功率", value: model.success_rate }
                        MiniStat { label: "P50 延迟", value: model.latency_p50 }
                    }
                }

                div { class: "border-t border-white/5" }

                // 画图展示
                div {
                    div { class: "mb-2 flex items-baseline justify-between",
                        p { class: "text-[11px] uppercase tracking-wider text-zinc-600", "趋势" }
                        span { class: "text-[11px] text-zinc-600", "近 24 小时" }
                    }
                    svg { class: "w-full", view_box: "0 0 200 56", preserve_aspect_ratio: "none",
                        defs {
                            linearGradient { id: "{gid}", x1: "0", y1: "0", x2: "0", y2: "1",
                                stop { offset: "0%", stop_color: "#ffffff", stop_opacity: "0.14" }
                                stop { offset: "100%", stop_color: "#ffffff", stop_opacity: "0" }
                            }
                        }
                        polygon { points: "{area}", fill: "url(#{gid})" }
                        polyline { points: "{line}", fill: "none", stroke: "#e4e4e7", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", vector_effect: "non-scaling-stroke" }
                        circle { cx: "{last_x}", cy: "{last_y}", r: "3", fill: "#09090b", stroke: "#e4e4e7", stroke_width: "2" }
                    }
                    // 热力条
                    div { class: "mt-2 flex gap-[3px]",
                        for (i, lv) in model.heat.iter().enumerate() {
                            span {
                                key: "{i}",
                                class: match lv {
                                    0 => "h-2.5 flex-1 rounded-[2px] bg-zinc-800",
                                    1 => "h-2.5 flex-1 rounded-[2px] bg-zinc-700",
                                    2 => "h-2.5 flex-1 rounded-[2px] bg-zinc-500",
                                    3 => "h-2.5 flex-1 rounded-[2px] bg-zinc-300",
                                    _ => "h-2.5 flex-1 rounded-[2px] bg-zinc-100",
                                },
                            }
                        }
                    }
                }
            }

            // ---- tab 2: 分组价格 ----
            if tab() == 1 {
                // Rows can grow — scroll past a few, header stays pinned on top.
                div { class: "max-h-64 overflow-y-auto scroll-subtle",
                    div { class: "sticky top-0 grid grid-cols-[minmax(0,2fr)_repeat(3,minmax(0,1fr))] items-baseline gap-x-3 bg-zinc-900 text-[11px] uppercase tracking-wider text-zinc-600",
                        span { "分组" }
                        span { class: "text-right", "输入" }
                        span { class: "text-right", "输出" }
                        span { class: "text-right", "缓存" }
                    }
                    div { class: "border-t border-white/5" }
                    for g in model.groups {
                        div { class: "mt-2 grid grid-cols-[minmax(0,2fr)_repeat(3,minmax(0,1fr))] items-baseline gap-x-3 border-b border-white/5 pb-2 text-sm",
                            span { class: "truncate font-medium text-zinc-200", "{g.name}" }
                            span { class: "text-right tabular-nums text-zinc-400", "{g.input}" }
                            span { class: "text-right tabular-nums text-zinc-400", "{g.output}" }
                            span { class: "text-right tabular-nums text-zinc-400", "{g.cache}" }
                        }
                    }
                }
            }

            // ---- tab 3: 待定 ----
            if tab() == 2 {
                div { class: "flex flex-1 items-center justify-center rounded-xl border border-dashed border-zinc-700 py-14 text-sm text-zinc-600",
                    "待定 · 预留位"
                }
            }
        }
    }
}

#[component]
fn MiniStat(label: &'static str, value: &'static str) -> Element {
    rsx! {
        div {
            p { class: "text-[11px] text-zinc-600", "{label}" }
            p { class: "mt-0.5 text-sm font-medium tabular-nums text-zinc-200", "{value}" }
        }
    }
}

/// 模型(演示)区块: #154 前的三内部 tab 卡片网格, 数据源为 admin-mock 演示数值。
/// 遵循页面响应式约定 (手机 1 栏 / 平板 md 3 栏 / Web xl 5 栏)。
#[component]
fn DemoModelsBoard() -> Element {
    let models = MODELS;

    rsx! {
        div { class: "space-y-4",
            "data-testid": "models-demo",
            role: "region",
            "aria-label": "模型（演示）",
            div { class: "flex items-baseline justify-between",
                h2 { class: "text-base font-semibold text-zinc-100", "模型" }
                span { class: "text-xs text-zinc-600", "mock · {models.len()} 个" }
            }
            div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 md:gap-4 lg:grid-cols-5",
                for m in models {
                    ModelCard { model: m.clone() }
                }
            }
        }
    }
}

/// 真实模型区块的卡片(one model = one card)。数据只来自真实 `ModelCardView` 字段,
/// 卡面结构恢复 #154 前的「模型展示卡」: 头部(名称/归属/状态) + 分区线 + 大数字统计块
/// + 三列 MiniStat。Width and flow come from the parent layout; the card is self-contained.
#[component]
fn RealModelCard(model: ModelCardView) -> Element {
    let card_cls = "flex flex-col gap-3 rounded-2xl border border-white/10 \
                    bg-gradient-to-b from-zinc-800/60 to-zinc-900/40 p-4 \
                    shadow-xl shadow-black/40 ring-1 ring-white/5 backdrop-blur-xl";

    // 状态:1 = 启用(与后端 status = 1 判定同口径),其余一律视为停用
    let (status_text, status_cls) = if model.status == 1 {
        ("启用", "text-emerald-400")
    } else {
        ("停用", "text-zinc-500")
    };

    rsx! {
        section { class: "{card_cls}",
            // Top bar: name + 归属/类型 + status(旧版头部槽位: 名称 + 厂商)
            header { class: "flex items-start justify-between gap-3",
                div { class: "min-w-0",
                    h3 { class: "truncate text-base font-semibold tracking-tight text-zinc-50", "{model.name}" }
                    p { class: "mt-0.5 text-xs text-zinc-500", "{model.model_type} · {model.owner}" }
                }
                span { class: "shrink-0 {status_cls} text-xs font-medium", "{status_text}" }
            }

            div { class: "border-t border-white/5" }

            // 大数字统计块(旧版「24H TOKENS」槽位): 累计调用为主数 + 三列关键指标。
            // 能力布尔为 false 时含后端缺省(serde default)的情形,用「—」表示未声明,不武断展示「不支持」。
            div {
                p { class: "text-[11px] uppercase tracking-wider text-zinc-600", "累计调用" }
                p { class: "mt-1 text-2xl font-semibold tabular-nums tracking-tight text-zinc-50", "{model.usage_count}" }
                div { class: "mt-3 grid grid-cols-3 gap-2",
                    RealMiniStat { label: "最大上下文", value: format!("{} tokens", model.max_tokens) }
                    RealMiniStat { label: "视觉", value: if model.is_vision { "支持".to_string() } else { "—".to_string() } }
                    RealMiniStat { label: "工具调用", value: if model.is_tool { "支持".to_string() } else { "—".to_string() } }
                }
            }
        }
    }
}

#[component]
fn RealMiniStat(label: &'static str, value: String) -> Element {
    rsx! {
        div {
            p { class: "text-[11px] text-zinc-600", "{label}" }
            p { class: "mt-0.5 text-sm font-medium tabular-nums text-zinc-200", "{value}" }
        }
    }
}

/// 模型页入口: 演示卡网格(区块一)在前, 真实模型列表(区块二)在后, 与排行榜页同构。
/// 真实列表数据来自 /api/models;后端单页上限 100 条,total 单独标注。
#[component]
pub fn ModelsPanel() -> Element {
    let mut models = use_signal(Vec::<ModelCardView>::new);
    let mut total = use_signal(|| 0i64);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut reload = use_signal(|| 0u32);

    use_effect(move || {
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            match api::list_models_api(&client).await {
                Ok((items, tot)) => {
                    models.set(items);
                    total.set(tot);
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    let data = models();
    let total_n = total();
    let is_loading = loading();
    let error = err();

    rsx! {
        div { class: "flex flex-col gap-6 md:gap-8",
            // 区块一: 模型(演示) — 恢复 #154 前的三内部 tab 卡片网格, 数据为 mock 演示数值
            DemoModelsBoard {}

            // ===== 区块二: 真实模型(真实 /api/models, #154 接线原样保留) =====
            div { class: "space-y-4",
                div { class: "flex items-baseline justify-between",
                    h2 { class: "text-base font-semibold text-zinc-100", "真实模型" }
                    div { class: "flex items-center gap-3",
                        // total 是库内总数;显示数受后端单页 100 条上限约束
                        span { class: "text-xs text-zinc-600", "共 {total_n} 个 · 显示 {data.len()} 个" }
                        button {
                            class: "shrink-0 rounded-xl border border-zinc-700 px-3 py-2 text-xs text-zinc-300 transition-colors hover:bg-zinc-800",
                            "data-testid": "refresh-models",
                            onclick: move |_| reload.set(reload() + 1),
                            "刷新"
                        }
                    }
                }
                section { "data-testid": "models-panel", role: "region", "aria-label": "模型列表",
                    if let Some(e) = error {
                        div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 px-4 py-6 text-center",
                            p { class: "text-sm text-red-300", "加载模型列表失败" }
                            p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                            button {
                                class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                                "data-testid": "retry-models",
                                onclick: move |_| reload.set(reload() + 1),
                                "重试"
                            }
                        }
                    } else if is_loading {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-10 text-center",
                            p { class: "text-zinc-400", "正在加载模型…" }
                        }
                    } else if data.is_empty() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-10 text-center",
                            p { class: "text-zinc-400", "后端无模型" }
                            p { class: "mt-1 text-xs text-zinc-600", "在管理端创建模型后这里会展示真实列表" }
                        }
                    } else {
                        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 md:gap-4 lg:grid-cols-5",
                            for m in data {
                                RealModelCard { model: m }
                            }
                        }
                    }
                }
            }
        }
    }
}
