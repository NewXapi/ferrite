use dioxus::prelude::*;

/// Per-group pricing row (分组价格 tab).
#[derive(Clone, Copy, PartialEq)]
pub struct GroupPrice {
    pub name: &'static str,
    pub input: &'static str,
    pub output: &'static str,
    pub cache: &'static str,
}

/// One model detail card's data. Mock for layout development.
#[derive(Clone, PartialEq)]
pub struct ModelInfo {
    pub name: &'static str,
    pub vendor: &'static str,
    pub description: &'static str,
    /// 输入 / 输出 / 缓存
    pub price_input: &'static str,
    pub price_output: &'static str,
    pub price_cache: &'static str,
    /// 数据展示
    pub tokens_24h: &'static str,
    pub cost_24h: &'static str,
    pub requests_24h: &'static str,
    pub success_rate: &'static str,
    pub latency_p50: &'static str,
    /// 画图展示: 24 sparkline points (0..=100) + 24 hourly heat levels (0..=4)
    pub trend: &'static [u8],
    pub heat: &'static [u8],
    /// 分组价格
    pub groups: &'static [GroupPrice],
}

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
    let line: String = pts.iter().map(|(x, y)| format!("{x:.1},{y:.1}")).collect::<Vec<_>>().join(" ");
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

const TREND_A: &[u8] = &[30, 38, 34, 46, 52, 47, 60, 58, 66, 61, 72, 78, 70, 82, 76, 88, 84, 92, 86, 95, 90, 97, 93, 100];
const HEAT_A: &[u8] = &[1, 0, 2, 1, 3, 2, 0, 4, 2, 1, 3, 0, 2, 4, 1, 2, 3, 0, 1, 2, 4, 3, 2, 1];
const TREND_B: &[u8] = &[70, 62, 66, 54, 60, 48, 56, 44, 52, 40, 46, 34, 42, 30, 38, 28, 36, 26, 34, 40, 30, 44, 36, 50];
const HEAT_B: &[u8] = &[3, 4, 2, 3, 1, 2, 4, 0, 2, 3, 1, 4, 2, 0, 3, 2, 1, 3, 4, 2, 1, 0, 2, 3];

const MOCK_MODELS: &[ModelInfo] = &[
    ModelInfo {
        name: "gpt-5.2",
        vendor: "openai",
        description: "旗舰通用模型，长上下文与工具调用强化。",
        price_input: "12.5",
        price_output: "100",
        price_cache: "1.25",
        tokens_24h: "70,514,208",
        cost_24h: "41.80",
        requests_24h: "3,204",
        success_rate: "99.2%",
        latency_p50: "1.4s",
        trend: TREND_A,
        heat: HEAT_A,
        groups: &[
            GroupPrice { name: "默认", input: "12.5", output: "100", cache: "1.25" },
            GroupPrice { name: "奶酪", input: "8", output: "10", cache: "1" },
            GroupPrice { name: "牛奶", input: "10", output: "80", cache: "1.1" },
            GroupPrice { name: "芝士", input: "15", output: "120", cache: "1.5" },
        ],
    },
    ModelInfo {
        name: "deepseek-chat",
        vendor: "deepseek",
        description: "高性价比对话模型，缓存命中价格极低。",
        price_input: "10",
        price_output: "20",
        price_cache: "0.1",
        tokens_24h: "48,713,006",
        cost_24h: "18.02",
        requests_24h: "5,861",
        success_rate: "98.7%",
        latency_p50: "0.9s",
        trend: TREND_B,
        heat: HEAT_B,
        groups: &[
            GroupPrice { name: "默认", input: "10", output: "20", cache: "0.1" },
            GroupPrice { name: "蓝纹奶酪", input: "12", output: "24", cache: "0.12" },
            GroupPrice { name: "芝士", input: "9", output: "18", cache: "0.09" },
        ],
    },
    ModelInfo {
        name: "claude-opus-4.7",
        vendor: "anthropic",
        description: "长程推理模型，代码与 Agent 任务首选。",
        price_input: "15",
        price_output: "75",
        price_cache: "1.5",
        tokens_24h: "31,206,544",
        cost_24h: "27.35",
        requests_24h: "2,047",
        success_rate: "99.5%",
        latency_p50: "2.1s",
        trend: TREND_A,
        heat: HEAT_B,
        groups: &[
            GroupPrice { name: "默认", input: "15", output: "75", cache: "1.5" },
        ],
    },
];

/// Preview grid（趋势 tab）: demonstrates the card in a real layout with 2 models.
#[component]
pub fn ModelsPanel() -> Element {
    rsx! {
        div { class: "space-y-4",
            div { class: "flex items-baseline justify-between",
                h2 { class: "text-base font-semibold text-zinc-100", "模型" }
                span { class: "text-xs text-zinc-600", "mock · {MOCK_MODELS.len()} 个" }
            }
            div { class: "grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3",
                for m in MOCK_MODELS {
                    ModelCard { model: m.clone() }
                }
            }
        }
    }
}
