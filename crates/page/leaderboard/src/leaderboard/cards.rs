//! 卡牌组件: 独立海报翻牌卡 (PosterImageCard) + 头牌翻牌卡 (MiniRadarCard).
//! 全部由 data 层的数值驱动; 样式只关心布局, 不关心数值.

use dioxus::prelude::*;

use super::data::{avg_norms, composite, dim_rank, dim_raw, key_stats, norms, ModelStat, DIMS};

/// 卡牌倾斜 hover 的通用几何: 返回 (rotateX, rotateY).
/// ponytail: 卡片尺寸硬编码 (246x368 / 460x320), 只是演示用.
fn tilt_from(x: f64, y: f64, w: f64, h: f64, ax: f64, ay: f64) -> (f64, f64) {
    let nx = (x / w - 0.5).clamp(-0.5, 0.5);
    let ny = (y / h - 0.5).clamp(-0.5, 0.5);
    (-ny * ax, nx * ay)
}

/// 排名角标的三档样式: (fill, glow filter, opacity)
fn badge_style(rank: usize, above_avg: bool, grad_id: &str) -> (String, &'static str, f64) {
    if rank <= 3 {
        (
            format!("url(#{grad_id})"),
            "drop-shadow(0 0 3px rgba(255,138,217,0.9)) drop-shadow(0 0 6px rgba(138,212,255,0.6))",
            1.0,
        )
    } else if above_avg {
        ("#ffffff".to_string(), "none", 1.0)
    } else {
        ("#71717a".to_string(), "none", 0.7)
    }
}

/// 六维雷达 svg 几何: 顶点坐标 + 多边形点串.
struct RadarGeo;

impl RadarGeo {
    const CX: f64 = 60.0;
    const CY: f64 = 58.0;
    const R: f64 = 44.0;

    fn pt(i: usize, v: f64) -> (f64, f64) {
        let a = -90f64.to_radians() + i as f64 * 60f64.to_radians();
        (Self::CX + Self::R * v * a.cos(), Self::CY + Self::R * v * a.sin())
    }

    fn poly(vals: &[f64; 6]) -> String {
        (0..6)
            .map(|i| {
                let (x, y) = Self::pt(i, vals[i]);
                format!("{x:.1},{y:.1}")
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// 立绘元素: 有图用图, 无图用首字母占位.
fn art_img(model: &'static ModelStat, extra_style: &str) -> Element {
    match model.art {
        Some(art) => rsx! {
            img {
                class: "absolute inset-0 h-full w-full object-cover",
                style: "{extra_style}",
                src: art,
                alt: "{model.name}",
            }
        },
        None => rsx! {
            span { class: "absolute inset-0 flex items-center justify-center text-9xl font-bold text-zinc-600/40",
                "{model.name.chars().next().unwrap_or('?')}"
            }
        },
    }
}

/// 三条关键数据行 (正面缩写 + 悬停全称).
#[component]
fn KeyStatRows(model: &'static ModelStat) -> Element {
    rsx! {
        div { class: "pointer-events-auto w-32 space-y-1",
            for s in key_stats(model) {
                div { class: "row-tip-anchor relative flex items-baseline justify-between text-[10px]",
                    span { class: "text-zinc-300 row-text", "{s.short}" }
                    span { class: "text-zinc-100 row-text", "{s.text}" }
                    div { class: "pointer-events-none absolute -top-1 left-0 z-10 -translate-y-full whitespace-nowrap rounded border border-white/15 bg-zinc-950/95 px-2 py-1 text-[10px] text-zinc-200 opacity-0 transition-opacity duration-200 row-tip",
                        "{s.full}"
                    }
                }
            }
        }
    }
}

/// 三条关键数据的横排统计 (背面底部).
#[component]
fn KeyStatFooter(model: &'static ModelStat) -> Element {
    rsx! {
        div { class: "mt-2 flex justify-between border-t border-white/10 pt-2",
            for s in key_stats(model) {
                div { class: "text-center",
                    p { class: "text-[10px] font-semibold text-zinc-100", "{s.text}" }
                    p { class: "text-[8px] uppercase tracking-wide text-zinc-500", "{s.short}" }
                }
            }
        }
    }
}

/// 独立翻牌海报卡: 正面 = 立绘 + 浓缩雷达 (角标/光点/均值虚线) + 三条数据;
/// 点击顺时针翻 180°, 背面 = 暗化立绘 + 六维横向直方图 + 综合分.
#[component]
pub fn PosterImageCard(rank: usize, model: &'static ModelStat) -> Element {
    let mut tilt = use_signal(|| (0.0f64, 0.0f64));
    let mut turns = use_signal(|| 0i32);

    let values = norms(model);
    let radar = RadarGeo::poly(&values);
    let self_path = format!("M {} Z", radar.replace(" ", " L "));
    let avg = avg_norms();
    let radar_avg = RadarGeo::poly(&avg);
    let ranks = dim_rank(model);
    let raw = dim_raw(model);
    let score = composite(model);
    let deg = turns() * 180;
    let grad_id = format!("rank-grad-{}", model.name);
    let path_id = format!("poster-path-{}", model.name);

    rsx! {
        article {
            class: "relative select-none self-start rounded-xl",
            style: "transform: perspective(1000px) rotateX({tilt().0:.2}deg) rotateY({tilt().1:.2}deg); transition: transform 0.5s cubic-bezier(0.22, 1, 0.36, 1);",
            onmousemove: move |evt| {
                let p = evt.data.element_coordinates();
                tilt.set(tilt_from(p.x, p.y, 246.0, 368.0, 6.0, 8.0));
            },
            onmouseleave: move |_| tilt.set((0.0, 0.0)),
            div {
                class: "poster-flip cursor-pointer",
                style: "aspect-ratio: 2 / 3;",
                onclick: move |_| turns.set(turns() + 1),
                div { class: "poster-flip-inner", style: "transform: rotateY({deg}deg)",
                    // 正面
                    div { class: "card-frame poster-flip-face overflow-hidden rounded-xl border border-white/15 bg-zinc-950 shadow-xl shadow-black/60",
                        {art_img(model, "")}
                        div { class: "card-vignette pointer-events-none absolute inset-0" }
                        div { class: "card-corner-shade pointer-events-none absolute inset-0" }
                        div { class: "pointer-events-none absolute bottom-3 left-3 flex w-[66%] flex-col gap-2",
                            // 浓缩雷达
                            div { class: "w-[58%]",
                                style: "filter: drop-shadow(0 2px 6px rgba(0,0,0,0.65))",
                                svg { class: "h-auto w-full", view_box: "2 0 116 116", preserve_aspect_ratio: "xMidYMid meet",
                                    for ring in 1..=3 {
                                        polygon { points: RadarGeo::poly(&[ring as f64 / 3.0; 6]), fill: "none", stroke: "rgba(255,255,255,0.24)", stroke_width: "1" }
                                    }
                                    polygon { points: radar_avg.clone(), fill: "none", stroke: "rgba(255,255,255,0.5)", stroke_width: "0.9", stroke_dasharray: "3 2.5" }
                                    path {
                                        id: "{path_id}",
                                        d: self_path.clone(),
                                        fill: "rgba(196,214,255,0.20)", stroke: "#f2f6ff", stroke_width: "1.8",
                                    }
                                    circle { r: "3", fill: "#e6eeff", opacity: "0.30",
                                        animateMotion { dur: "5s", repeat_count: "indefinite", mpath { href: "#{path_id}" } }
                                    }
                                    circle { r: "1.4", fill: "#ffffff",
                                        animateMotion { dur: "5s", repeat_count: "indefinite", mpath { href: "#{path_id}" } }
                                    }
                                    defs {
                                        linearGradient { id: "{grad_id}", x1: "0%", y1: "0%", x2: "100%", y2: "100%",
                                            stop { offset: "0%", stop_color: "#ffd479" }
                                            stop { offset: "45%", stop_color: "#ff8ad9" }
                                            stop { offset: "100%", stop_color: "#8ad4ff" }
                                        }
                                    }
                                    for i in 0..6 {
                                        {
                                            // 角标放在顶点所在的下一级环线上, 封顶最外环.
                                            let ring = ((values[i] * 3.0).floor() + 1.0).min(3.0) / 3.0;
                                            let (bx, by) = RadarGeo::pt(i, ring);
                                            let (fill, glow, dim) = badge_style(ranks[i], values[i] > avg[i] + 1e-9, &grad_id);
                                            rsx! {
                                                text {
                                                    x: "{bx:.1}", y: "{by:.1}", text_anchor: "middle",
                                                    font_size: "9", font_weight: "700", fill: "{fill}",
                                                    style: "paint-order: stroke; stroke: rgba(0,0,0,0.9); stroke-width: 2.6px; stroke-linejoin: round; filter: {glow}; opacity: {dim};",
                                                    "#{ranks[i]}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            KeyStatRows { model }
                        }
                    }
                    // 背面
                    div { class: "card-frame poster-flip-face poster-flip-back overflow-hidden rounded-xl border border-white/15 bg-zinc-950 shadow-xl shadow-black/60",
                        {art_img(model, "filter: brightness(0.28); transform: scale(1.02)")}
                        div { class: "card-vignette pointer-events-none absolute inset-0" }
                        div { class: "absolute inset-0 flex flex-col px-4 pb-3 pt-3",
                            div { class: "flex items-baseline justify-between",
                                h3 { class: "text-sm font-semibold text-white row-text", "{model.name}" }
                                span { class: "text-[10px] font-semibold text-zinc-400", "#{rank}" }
                            }
                            p { class: "mt-0.5 text-[10px] leading-snug text-zinc-400", "{model.desc}" }
                            div { class: "my-2.5 flex items-center gap-3",
                                div { class: "h-px flex-1 bg-white/12" }
                                div { class: "flex items-baseline gap-1.5",
                                    span { class: "text-2xl font-bold tracking-tight text-white row-text", "{score:.1}" }
                                    span { class: "text-[9px] font-medium uppercase tracking-widest text-zinc-500", "Score" }
                                }
                                div { class: "h-px flex-1 bg-white/12" }
                            }
                            div { class: "flex-1 space-y-1.5",
                                for i in 0..6 {
                                    div { class: "flex items-center gap-2",
                                        span { class: "w-9 text-[11px] font-medium leading-none text-zinc-300", "{DIMS[i]}" }
                                        div { class: "h-1 flex-1 overflow-hidden rounded-full bg-white/15",
                                            div { class: "h-full rounded-full bg-zinc-100", style: "width: {values[i] * 100.0:.0}%" }
                                        }
                                        span { class: "w-14 text-right text-[11px] font-medium leading-none text-zinc-100", "{raw[i]}" }
                                        span {
                                            class: if ranks[i] <= 3 { "w-6 text-right text-[8px] font-bold text-amber-300" } else { "w-6 text-right text-[8px] text-zinc-600" },
                                            "#{ranks[i]}"
                                        }
                                    }
                                }
                            }
                            KeyStatFooter { model }
                        }
                    }
                }
            }
        }
    }
}

/// 头牌翻牌卡: 左侧翻牌立绘 (正面立绘 / 背面六维明细), 右侧信息面板 (名牌 + 描述).
#[component]
pub fn MiniRadarCard(
    rank: usize,
    /// 立绘斜角 (度), 0 = 直立
    lean: f64,
    model: &'static ModelStat,
) -> Element {
    let mut flipped = use_signal(|| false);
    let mut tilt = use_signal(|| (0.0f64, 0.0f64));
    let flip_cls = if flipped() { "poster-flip-inner is-flipped" } else { "poster-flip-inner" };
    let initial = model.name.chars().next().unwrap_or('?');
    let raw = dim_raw(model);
    let score = composite(model);

    rsx! {
        div {
            class: "card-tilt relative self-start rounded-xl border border-zinc-800 bg-zinc-900 p-5",
            style: "transform: perspective(900px) rotateX({tilt().0:.2}deg) rotateY({tilt().1:.2}deg)",
            onmousemove: move |evt| {
                let p = evt.data.element_coordinates();
                tilt.set(tilt_from(p.x, p.y, 460.0, 320.0, 8.0, 10.0));
            },
            onmouseleave: move |_| tilt.set((0.0, 0.0)),
            div { class: "flex gap-5",
                // 左: 翻牌立绘
                div { class: "w-1/2 shrink-0 py-3",
                    style: "transform: rotate({lean}deg)",
                    div { class: "poster-flip poster-flip-portrait",
                        onclick: move |_| flipped.set(!flipped()),
                        div { class: "{flip_cls}",
                            div { class: "card-frame card-frosted poster-flip-face overflow-hidden rounded-xl border border-zinc-700 shadow-xl shadow-black/40",
                                {art_img(model, "")}
                            }
                            div { class: "card-frame poster-flip-face poster-flip-back overflow-hidden rounded-xl border border-zinc-700 bg-zinc-950 p-4",
                                div { class: "flex items-baseline justify-between",
                                    h3 { class: "text-sm font-semibold text-zinc-100", "{model.name}" }
                                    span { class: "text-[10px] text-zinc-500", "#{rank}" }
                                }
                                p { class: "mt-1 text-[10px] text-zinc-500", "{model.desc}" }
                                div { class: "mt-3 space-y-2",
                                    for i in 0..6 {
                                        div { class: "flex items-baseline justify-between border-b border-zinc-800/60 pb-1",
                                            span { class: "text-[11px] text-zinc-500", "{DIMS[i]}" }
                                            span { class: "text-[11px] text-zinc-200", "{raw[i]}" }
                                        }
                                    }
                                }
                                div { class: "mt-3 flex items-baseline justify-between",
                                    span { class: "text-[11px] text-zinc-500", "综合分" }
                                    span { class: "text-sm font-semibold text-zinc-100", "{score:.1}" }
                                }
                            }
                        }
                    }
                }
                // 右: 信息面板
                div { class: "flex min-w-0 flex-1 flex-col justify-center gap-2",
                    div { class: "rounded-lg border border-white/10 bg-zinc-950/50 px-2.5 py-2",
                        div { class: "flex items-center gap-2",
                            span { class: "flex h-5 w-5 shrink-0 items-center justify-center rounded-full border border-zinc-600 bg-zinc-800/80 text-[9px] font-medium text-zinc-400", "{initial}" }
                            span { class: "min-w-0 truncate text-xs font-medium tracking-wide text-zinc-200", "{model.name}" }
                            span { class: "ml-auto text-[10px] font-medium italic text-zinc-500", "#{rank}" }
                        }
                    }
                    p { class: "text-[11px] leading-relaxed text-zinc-500", "{model.desc}" }
                }
            }
        }
    }
}
