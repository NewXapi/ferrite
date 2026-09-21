//! 立绘海报卡(PosterCard) — 抽象自 admin-page-overview `leaderboard::cards::PosterImageCard`。
//!
//! 布局: 正面 = 立绘 + 浓缩六维雷达 SVG(三档底环 / 均值虚线 / 自身多边形 / 双光点
//! animateMotion 动画 / 六维排名角标) + 名称名牌 + "#N" 排名角标 + 关键数据行;
//! 点击顺时针翻 180°, 背面 = 暗化立绘 + 综合分 + 六维横向直方图 + 关键数据横排。
//! 外层 246x368 鼠标倾斜 hover(与原实现一致, 仅演示近似)。
//!
//! 样式基座: admin-web entry.css 的 poster-flip / card-frame / card-vignette /
//! card-corner-shade / row-text / row-tip 类 + Tailwind 原子类。外层容器无 border,
//! hover 边框变亮不在组件内追加(加 border 会改变观感), 需由宿主在 entry.css 的
//! `.poster-flip:hover` 语境处理。
//!
//! DOM id 注意: 雷达渐变 id 与动画路径 id 由 `name` 派生(`rank-grad-{name}` /
//! `poster-path-{name}`, 与原实现一致); 同屏同名卡会共享 id。

use dioxus::prelude::*;

use super::art_img;

/// 关键数据行: 正面缩写行(悬停展开全称)与背面底部横排共用。
#[derive(Clone, PartialEq)]
pub struct KeyStatLine {
    /// 缩写标签(如 "Token" / "Requests" / "Share")。
    pub short: String,
    /// 悬停展开的全称文案(如 "Total Token: 3.2B")。
    pub full: String,
    /// 展示值文本(如 "3.2B")。
    pub text: String,
}

/// 立绘海报卡: 正面 = 立绘 + 浓缩雷达 + 关键数据; 点击顺时针翻 180°, 背面 = 暗化立绘
/// + 六维横向直方图 + 综合分。
///
/// # 参数
/// - `rank`: 排名(1 = 最强); <= 3 时排名角标金色发光, 其余常规灰。
/// - `name`: 名称(名牌 / 背面标题 / img alt / 雷达 DOM id 后缀 / 占位首字符来源)。
/// - `desc`: 背面描述文字。
/// - `art`: 立绘资产; None = 名称首字符占位(正背面一致)。
/// - `radar_values`: 六维雷达值(归一 0..=1, 顺序对应六边形顶点)。
/// - `radar_avg`: 全体平均雷达值(均值虚线参照, 0..=1)。
/// - `dim_ranks`: 六维名次(1 = 最强), 雷达角标文字与背面行尾 "#N"。
/// - `dim_labels`: 六维标签(背面直方图行首, 如 "速度")。
/// - `dim_raws`: 六维展示值(背面直方图行中, 已由调用方格式化, 如 "92" / "$3.00")。
/// - `score`: 综合分(背面大数字, 格式化到 1 位小数)。
/// - `key_stats`: 关键数据行(正面左下三行 + 背面底部横排共用, 原实现固定 3 行)。
/// - `testid` / `aria_label`: 可选透传; `aria_label` 给出时容器同时带 `role="region"`。
///
/// # 示例
/// ```ignore
/// rsx! {
///     PosterCard {
///         rank: 1,
///         name: "gpt-5.6-sol".into(),
///         desc: "OpenAI 旗舰推理模型".into(),
///         art: None,
///         radar_values: [0.7, 0.8, 0.4, 1.0, 0.9, 1.0],
///         radar_avg: [0.6, 0.6, 0.5, 0.9, 0.8, 0.7],
///         dim_ranks: [3, 2, 5, 1, 2, 1],
///         dim_labels: std::array::from_fn(|_| "速度".to_string()),
///         dim_raws: std::array::from_fn(|_| "92".to_string()),
///         score: 78.9,
///         key_stats: vec![KeyStatLine { short: "Token".into(), full: "Total Token: 3.2B".into(), text: "3.2B".into() }],
///     }
/// }
/// ```
#[component]
pub fn PosterCard(
    /// 排名(1 = 最强); <= 3 时排名角标金色发光。
    rank: usize,
    /// 名称: 名牌 / 背面标题 / img alt / 雷达 DOM id 后缀 / 占位首字符来源。
    name: String,
    /// 背面描述文字。
    desc: String,
    /// 立绘资产; None = 名称首字符占位(正背面一致)。
    art: Option<Asset>,
    /// 六维雷达值(归一 0..=1, 顺序对应六边形顶点)。
    radar_values: [f64; 6],
    /// 全体平均雷达值(均值虚线参照, 0..=1)。
    radar_avg: [f64; 6],
    /// 六维名次(1 = 最强): 雷达角标文字与背面行尾 "#N"。
    dim_ranks: [usize; 6],
    /// 六维标签(背面直方图行首)。
    dim_labels: [String; 6],
    /// 六维展示值(背面直方图行中, 已由调用方格式化)。
    dim_raws: [String; 6],
    /// 综合分(背面大数字, 格式化到 1 位小数)。
    score: f64,
    /// 关键数据行(正面左下三行 + 背面底部横排共用)。
    key_stats: Vec<KeyStatLine>,
    /// 可选透传: 容器 data-testid, 默认不渲染。
    #[props(default)]
    testid: Option<String>,
    /// 可选透传: 容器 aria-label, 默认不渲染; 给出时容器同时带 role="region"。
    #[props(default)]
    aria_label: Option<String>,
) -> Element {
    let mut tilt = use_signal(|| (0.0f64, 0.0f64));
    let mut turns = use_signal(|| 0i32);

    let radar = RadarGeo::poly(&radar_values);
    let self_path = format!("M {} Z", radar.replace(" ", " L "));
    let avg_poly = RadarGeo::poly(&radar_avg);
    let deg = turns() * 180;
    let is_flipped = (turns() % 2) != 0;
    let grad_id = format!("rank-grad-{}", name);
    let path_id = format!("poster-path-{}", name);
    let region_role = aria_label.as_ref().map(|_| "region");

    rsx! {
        article {
            class: "relative select-none self-start rounded-xl",
            "data-testid": testid,
            role: region_role,
            "aria-label": aria_label,
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
                    div {
                        class: "card-frame poster-flip-face overflow-hidden rounded-xl border border-white/15 bg-zinc-950 shadow-xl shadow-black/60",
                        style: if is_flipped { "visibility: hidden; opacity: 0; pointer-events: none;" } else { "visibility: visible; opacity: 1;" },
                        {art_img(art, &name, "")}
                        div { class: "card-vignette pointer-events-none absolute inset-0" }
                        div { class: "card-corner-shade pointer-events-none absolute inset-0" }

                        // 顶部阴影层 (加深凸显标题)
                        div { class: "pointer-events-none absolute inset-x-0 top-0 h-28 bg-gradient-to-b from-black/85 via-black/40 to-transparent rounded-t-xl" }

                        // 底部阴影层 (加深凸显雷达与数据)
                        div { class: "pointer-events-none absolute inset-x-0 bottom-0 h-44 bg-gradient-to-t from-black/95 via-black/60 to-transparent rounded-b-xl" }

                        // 顶部：模型名称与排名角标
                        div { class: "pointer-events-none absolute top-3 inset-x-3 flex items-center justify-between",
                            div { class: "flex items-center gap-1.5 rounded-full border border-white/20 bg-black/60 px-2.5 py-1",
                                span { class: "text-xs font-bold tracking-wide text-white row-text", "{name}" }
                            }
                            div { class: "flex items-center justify-center rounded-full border border-white/20 bg-black/60 px-2 py-0.5",
                                span {
                                    class: if rank <= 3 { "text-xs font-extrabold text-amber-300 drop-shadow-[0_0_8px_rgba(251,191,36,0.6)]" } else { "text-xs font-semibold text-zinc-300" },
                                    "#{rank}"
                                }
                            }
                        }

                        // 底部：浓缩雷达居左、关键数据蒙版贴右下角
                        // (维护者批注 2026-09-21: 数据蒙版没有贴近右下角, 调整;
                        //  原实现整体钉左下、右下角留给立绘黄金徽章)
                        div { class: "pointer-events-none absolute inset-x-3 bottom-3 flex items-end justify-between gap-2",
                            // 浓缩雷达
                            div { class: "w-28 shrink-0",
                                style: "filter: drop-shadow(0 2px 6px rgba(0,0,0,0.65))",
                                svg { class: "h-auto w-full", view_box: "2 0 116 116", preserve_aspect_ratio: "xMidYMid meet",
                                    for ring in 1..=3 {
                                        polygon { points: RadarGeo::poly(&[ring as f64 / 3.0; 6]), fill: "none", stroke: "rgba(255,255,255,0.24)", stroke_width: "1" }
                                    }
                                    polygon { points: avg_poly, fill: "none", stroke: "rgba(255,255,255,0.5)", stroke_width: "0.9", stroke_dasharray: "3 2.5" }
                                    path {
                                        id: "{path_id}",
                                        d: self_path,
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
                                            let ring = ((radar_values[i] * 3.0).floor() + 1.0).min(3.0) / 3.0;
                                            let (bx, by) = RadarGeo::pt(i, ring);
                                            let (fill, glow, dim) = badge_style(dim_ranks[i], radar_values[i] > radar_avg[i] + 1e-9, &grad_id);
                                            rsx! {
                                                text {
                                                    x: "{bx:.1}", y: "{by:.1}", text_anchor: "middle",
                                                    font_size: "9", font_weight: "700", fill: "{fill}",
                                                    style: "paint-order: stroke; stroke: rgba(0,0,0,0.9); stroke-width: 2.6px; stroke-linejoin: round; filter: {glow}; opacity: {dim};",
                                                    "#{dim_ranks[i]}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            KeyStatRows { stats: key_stats.clone() }
                        }
                    }
                    // 背面
                    div {
                        class: "card-frame poster-flip-face poster-flip-back overflow-hidden rounded-xl border border-white/15 bg-zinc-950 shadow-xl shadow-black/60",
                        style: if !is_flipped { "visibility: hidden; opacity: 0; pointer-events: none;" } else { "visibility: visible; opacity: 1;" },
                        {art_img(art, &name, "filter: brightness(0.28); transform: scale(1.02)")}
                        div { class: "card-vignette pointer-events-none absolute inset-0" }
                        div { class: "absolute inset-0 flex flex-col px-4 pb-3 pt-3",
                            div { class: "flex items-baseline justify-between",
                                h3 { class: "text-sm font-semibold text-white row-text", "{name}" }
                                span { class: "text-[10px] font-semibold text-zinc-400", "#{rank}" }
                            }
                            p { class: "mt-0.5 text-[10px] leading-snug text-zinc-400", "{desc}" }
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
                                        span { class: "w-9 text-[11px] font-medium leading-none text-zinc-300", "{dim_labels[i]}" }
                                        div { class: "h-1 flex-1 overflow-hidden rounded-full bg-white/15",
                                            div { class: "h-full rounded-full bg-zinc-100", style: "width: {radar_values[i] * 100.0:.0}%" }
                                        }
                                        span { class: "w-14 text-right text-[11px] font-medium leading-none text-zinc-100", "{dim_raws[i]}" }
                                        span {
                                            class: if dim_ranks[i] <= 3 { "w-6 text-right text-[8px] font-bold text-amber-300" } else { "w-6 text-right text-[8px] text-zinc-600" },
                                            "#{dim_ranks[i]}"
                                        }
                                    }
                                }
                            }
                            KeyStatFooter { stats: key_stats }
                        }
                    }
                }
            }
        }
    }
}

/// 三条关键数据行 (正面缩写 + 悬停全称)。
#[component]
fn KeyStatRows(stats: Vec<KeyStatLine>) -> Element {
    rsx! {
        div { class: "pointer-events-auto rounded-lg border border-white/10 bg-black/60 px-2 py-1.5 space-y-0.5",
            for s in stats.iter() {
                div { class: "row-tip-anchor relative flex items-baseline justify-between gap-2 text-[10px]",
                    span { class: "text-zinc-400 font-medium", "{s.short}" }
                    span { class: "text-zinc-100 font-mono font-bold row-text text-right", "{s.text}" }
                    div { class: "pointer-events-none absolute -top-1 left-0 z-20 -translate-y-full whitespace-nowrap rounded border border-white/15 bg-zinc-950/95 px-2 py-1 text-[10px] text-zinc-200 opacity-0 transition-opacity duration-200 row-tip shadow-lg",
                        "{s.full}"
                    }
                }
            }
        }
    }
}

/// 关键数据的横排统计 (背面底部)。
#[component]
fn KeyStatFooter(stats: Vec<KeyStatLine>) -> Element {
    rsx! {
        div { class: "mt-2 flex justify-between border-t border-white/10 pt-2",
            for s in stats.iter() {
                div { class: "text-center",
                    p { class: "text-[10px] font-semibold text-zinc-100", "{s.text}" }
                    p { class: "text-[8px] uppercase tracking-wide text-zinc-500", "{s.short}" }
                }
            }
        }
    }
}

/// 排名角标的三档样式: (fill, glow filter, opacity)。
/// rank <= 3 走渐变填充 + 粉蓝双色辉光; 高于均值白色; 其余灰且降透明。
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

/// 六维雷达 svg 几何: 顶点坐标 + 多边形点串。
struct RadarGeo;

impl RadarGeo {
    const CX: f64 = 60.0;
    const CY: f64 = 58.0;
    const R: f64 = 44.0;

    fn pt(i: usize, v: f64) -> (f64, f64) {
        let a = -90f64.to_radians() + i as f64 * 60f64.to_radians();
        (
            Self::CX + Self::R * v * a.cos(),
            Self::CY + Self::R * v * a.sin(),
        )
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

/// 卡牌倾斜 hover 的通用几何: 鼠标坐标 -> (rotateX, rotateY)。
/// 卡片尺寸硬编码 (PosterCard 246x368, 原实现如此), 仅为演示近似。
fn tilt_from(x: f64, y: f64, w: f64, h: f64, ax: f64, ay: f64) -> (f64, f64) {
    let nx = (x / w - 0.5).clamp(-0.5, 0.5);
    let ny = (y / h - 0.5).clamp(-0.5, 0.5);
    (-ny * ax, nx * ay)
}
