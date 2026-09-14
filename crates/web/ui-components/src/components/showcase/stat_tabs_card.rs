//! 三内部 tab 展示卡(StatTabsCard) — 抽象自 admin-page-overview `models::ModelCard`。
//!
//! 布局: 卡头(标题/副标题 + 三个图标式内部 tab 切换, TabGlyph) + 三个互斥内部 tab:
//! - 概览: 价格三元组行 / 描述 / 大数字统计块(标签 + 主数值 + 脚注) + 三列迷你统计 /
//!   趋势 sparkline(200x56 viewBox, 点列取值 0..=100) + 热力条(档位 0..=4)。
//! - 分组价格: 分组报价表(粘性表头, 行多可滚动, scroll-subtle)。
//! - 待定: 虚线占位(「待定 · 预留位」, 原样保留)。
//!
//! 样式基座: admin-web entry.css 的 scroll-subtle 类 + Tailwind 原子类;
//! hover 边框变亮通过 [`super::HOVER_BORDER_BRIGHT`] 挂在外层容器。

use dioxus::prelude::*;

use super::HOVER_BORDER_BRIGHT;

/// 价格三元组(输入 / 输出 / 缓存), 概览价格行与分组报价行共用。
#[derive(Clone, PartialEq)]
pub struct PriceTriple {
    /// 输入价展示文本(如 "12.5")。
    pub input: String,
    /// 输出价展示文本(如 "100")。
    pub output: String,
    /// 缓存价展示文本(如 "1.25")。
    pub cache: String,
}

/// 分组报价行(分组价格 tab 的每一行)。
#[derive(Clone, PartialEq)]
pub struct GroupPriceRow {
    /// 分组名(截断显示)。
    pub name: String,
    /// 该组的输入 / 输出 / 缓存价。
    pub price: PriceTriple,
}

/// 大数字统计块(概览 tab): 标签 + 主数值 + 可选脚注。
#[derive(Clone, PartialEq)]
pub struct HeadlineStat {
    /// 大写小标签(如 "24h Tokens")。
    pub label: String,
    /// 主数值(如 "70,514,208")。
    pub value: String,
    /// 主数值下方脚注行(如 "$41.80"); None = 不渲染脚注行(原实现恒有, 由调用方传 Some)。
    pub sub: Option<String>,
}

/// 迷你统计条目(概览 tab 的三列小字)。
#[derive(Clone, PartialEq)]
pub struct MiniStatItem {
    /// 标签(如 "请求" / "成功率" / "P50 延迟")。
    pub label: String,
    /// 展示值(如 "3,204" / "99.2%" / "1.4s")。
    pub value: String,
}

/// One card = one entity. Three internal tabs: 概览 / 分组价格 / 待定.
/// Width and flow come from the parent layout; the card is self-contained.
///
/// # 参数
/// - `title` / `subtitle`: 卡头标题与副标题(原实现为模型名 / 厂商)。
/// - `description`: 概览 tab 的描述文字。
/// - `price`: 概览价格行(输入 / 输出 / 缓存三元组, 行首标签固定「输入/输出/缓存」)。
/// - `headline`: 大数字统计块(标签 + 主数值 + 可选脚注)。
/// - `mini_stats`: 三列迷你统计(原实现为 请求 / 成功率 / P50 延迟)。
/// - `trend`: sparkline 点列(0..=100, 原实现 24 点; 需至少 1 点, 空序列会如原实现一样 panic)。
/// - `heat`: 热力条档位(0..=4, 原实现 24 格)。
/// - `groups`: 分组价格 tab 的报价行。
/// - `testid` / `aria_label`: 可选透传; `aria_label` 给出时容器同时带 `role="region"`。
///
/// # 示例
/// ```ignore
/// rsx! {
///     StatTabsCard {
///         title: "gpt-5.2".into(),
///         subtitle: "openai".into(),
///         description: "旗舰通用模型".into(),
///         price: PriceTriple { input: "12.5".into(), output: "100".into(), cache: "1.25".into() },
///         headline: HeadlineStat { label: "24h Tokens".into(), value: "70,514,208".into(), sub: Some("$41.80".into()) },
///         mini_stats: vec![MiniStatItem { label: "请求".into(), value: "3,204".into() }],
///         trend: vec![30, 38, 46],
///         heat: vec![1, 0, 2],
///         groups: vec![GroupPriceRow { name: "默认".into(), price: PriceTriple { input: "12.5".into(), output: "100".into(), cache: "1.25".into() } }],
///     }
/// }
/// ```
#[component]
pub fn StatTabsCard(
    /// 卡头标题(截断显示)。
    title: String,
    /// 标题下方副标题(原实现为厂商)。
    subtitle: String,
    /// 概览 tab 的描述文字。
    description: String,
    /// 概览价格行: 输入 / 输出 / 缓存三元组。
    price: PriceTriple,
    /// 大数字统计块(标签 + 主数值 + 可选脚注)。
    headline: HeadlineStat,
    /// 三列迷你统计(标签 + 数值)。
    mini_stats: Vec<MiniStatItem>,
    /// 趋势 sparkline 点列(0..=100, 原实现 24 点; 需至少 1 点)。
    trend: Vec<u8>,
    /// 热力条档位(0..=4, 原实现 24 格)。
    heat: Vec<u8>,
    /// 分组价格 tab 的报价行。
    groups: Vec<GroupPriceRow>,
    /// 可选透传: 容器 data-testid, 默认不渲染。
    #[props(default)]
    testid: Option<String>,
    /// 可选透传: 容器 aria-label, 默认不渲染; 给出时容器同时带 role="region"。
    #[props(default)]
    aria_label: Option<String>,
) -> Element {
    let mut tab = use_signal(|| 0u8);

    // Sparkline geometry (viewBox 200x56)
    let n = trend.len().max(2) as f32;
    let pts: Vec<(f32, f32)> = trend
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
    let gid = format!("fill-{}", title.replace(['.', '-'], "_"));
    let hover_cls = HOVER_BORDER_BRIGHT;
    let region_role = aria_label.as_ref().map(|_| "region");

    let card_cls = "flex flex-col gap-3 rounded-2xl border border-white/10 \
                    bg-gradient-to-b from-zinc-800/60 to-zinc-900/40 p-4 \
                    shadow-xl shadow-black/40 ring-1 ring-white/5 backdrop-blur-xl";

    rsx! {
        section { class: "{card_cls} {hover_cls}",
            "data-testid": testid,
            role: region_role,
            "aria-label": aria_label,
            // Top bar: title + internal tab switcher
            header { class: "flex items-start justify-between gap-3",
                div { class: "min-w-0",
                    h3 { class: "truncate text-base font-semibold tracking-tight text-zinc-50", "{title}" }
                    p { class: "mt-0.5 text-xs text-zinc-500", "{subtitle}" }
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
                    span { class: "text-zinc-500", "输入 " b { class: "font-semibold tabular-nums text-zinc-100", "{price.input}" } }
                    span { class: "text-zinc-500", "输出 " b { class: "font-semibold tabular-nums text-zinc-100", "{price.output}" } }
                    span { class: "text-zinc-500", "缓存 " b { class: "font-semibold tabular-nums text-zinc-100", "{price.cache}" } }
                }
                p { class: "text-xs text-zinc-500", "{description}" }

                div { class: "border-t border-white/5" }

                // 数据展示
                div {
                    p { class: "text-[11px] uppercase tracking-wider text-zinc-600", "{headline.label}" }
                    p { class: "mt-1 text-2xl font-semibold tabular-nums tracking-tight text-zinc-50", "{headline.value}" }
                    {headline.sub.as_ref().map(|sub| rsx! {
                        p { class: "mt-0.5 text-xs tabular-nums text-zinc-500", "{sub}" }
                    })}
                    div { class: "mt-3 grid grid-cols-3 gap-2",
                        for ms in mini_stats.iter() {
                            MiniStat { label: ms.label.clone(), value: ms.value.clone() }
                        }
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
                        for (i, lv) in heat.iter().enumerate() {
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
                    for g in groups.iter() {
                        div { class: "mt-2 grid grid-cols-[minmax(0,2fr)_repeat(3,minmax(0,1fr))] items-baseline gap-x-3 border-b border-white/5 pb-2 text-sm",
                            span { class: "truncate font-medium text-zinc-200", "{g.name}" }
                            span { class: "text-right tabular-nums text-zinc-400", "{g.price.input}" }
                            span { class: "text-right tabular-nums text-zinc-400", "{g.price.output}" }
                            span { class: "text-right tabular-nums text-zinc-400", "{g.price.cache}" }
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

/// 三列迷你统计的单格 (标签 + 数值)。
#[component]
fn MiniStat(label: String, value: String) -> Element {
    rsx! {
        div {
            p { class: "text-[11px] text-zinc-600", "{label}" }
            p { class: "mt-0.5 text-sm font-medium tabular-nums text-zinc-200", "{value}" }
        }
    }
}
