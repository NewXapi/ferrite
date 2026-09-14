//! 头牌翻牌卡(RadarFlipCard) — 抽象自 admin-page-overview `leaderboard::cards::MiniRadarCard`。
//!
//! 布局: 左侧翻牌立绘(正面 card-frosted 磨砂立绘 / 背面六维明细 + 综合分, 点击翻转),
//! 右侧信息面板(首字符徽标 + 名称 + "#N" 排名 + 描述)。外层 card-tilt 提供鼠标跟随
//! 倾斜(几何 460x320 / 幅度 8,10, 与原实现一致, 仅演示近似)。
//!
//! 样式基座: admin-web entry.css 的 poster-flip / card-frame / card-frosted / card-tilt
//! 类 + Tailwind 原子类; hover 边框变亮通过 [`super::HOVER_BORDER_BRIGHT`] 挂在外层容器。

use dioxus::prelude::*;

use super::{HOVER_BORDER_BRIGHT, art_img};

/// 六维明细行: 背面每行渲染一个「标签 + 展示值」对。
#[derive(Clone, PartialEq)]
pub struct DimRow {
    /// 维度标签(如 "速度" / "性价比")。
    pub label: String,
    /// 展示值(已由调用方格式化, 如 "92" / "$3.00")。
    pub value: String,
}

/// 头牌翻牌卡: 左侧翻牌立绘 (正面立绘 / 背面六维明细), 右侧信息面板 (名牌 + 描述)。
///
/// # 参数
/// - `rank`: 排名(1 = 最强), 渲染为 "#N" 文本(背面标题行与右侧面板各一次)。
/// - `lean`: 立绘斜角(度), 0 = 直立; 原实现按奇偶位交替 ±4°。
/// - `name`: 名称(名牌 / 背面标题 / 占位首字符来源)。
/// - `desc`: 一句话描述(背面与右侧面板各渲染一次)。
/// - `art`: 立绘资产; None = 名称首字符占位。
/// - `dim_rows`: 背面六维明细行(标签 + 展示值), 原实现固定 6 行。
/// - `score`: 综合分(原实现 = 六维归一均值 x100), 背面底部, 格式化到 1 位小数。
/// - `testid` / `aria_label`: 可选透传; `aria_label` 给出时容器同时带 `role="region"`。
///
/// # 示例
/// ```ignore
/// rsx! {
///     RadarFlipCard {
///         rank: 1,
///         lean: -4.0,
///         name: "gpt-5.6-sol".into(),
///         desc: "OpenAI 旗舰推理模型".into(),
///         art: None,
///         dim_rows: vec![DimRow { label: "速度".into(), value: "92".into() }],
///         score: 78.9,
///     }
/// }
/// ```
#[component]
pub fn RadarFlipCard(
    /// 排名(1 = 最强), 渲染为 "#N"。
    rank: usize,
    /// 立绘斜角(度), 0 = 直立。
    lean: f64,
    /// 名称(名牌 / 背面标题 / 占位首字符来源)。
    name: String,
    /// 一句话描述(背面与右侧面板各渲染一次)。
    desc: String,
    /// 立绘资产; None = 名称首字符占位。
    art: Option<Asset>,
    /// 背面六维明细行(标签 + 展示值), 原实现固定 6 行。
    dim_rows: Vec<DimRow>,
    /// 综合分, 背面底部(格式化到 1 位小数)。
    score: f64,
    /// 可选透传: 容器 data-testid, 默认不渲染。
    #[props(default)]
    testid: Option<String>,
    /// 可选透传: 容器 aria-label, 默认不渲染; 给出时容器同时带 role="region"。
    #[props(default)]
    aria_label: Option<String>,
) -> Element {
    let mut flipped = use_signal(|| false);
    let mut tilt = use_signal(|| (0.0f64, 0.0f64));
    let flip_cls = if flipped() {
        "poster-flip-inner is-flipped"
    } else {
        "poster-flip-inner"
    };
    let initial = name.chars().next().unwrap_or('?');
    let hover_cls = HOVER_BORDER_BRIGHT;
    let region_role = aria_label.as_ref().map(|_| "region");

    rsx! {
        div {
            class: "card-tilt relative self-start rounded-xl border border-zinc-800 bg-zinc-900 p-5 {hover_cls}",
            "data-testid": testid,
            role: region_role,
            "aria-label": aria_label,
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
                                {art_img(art, &name, "")}
                            }
                            div { class: "card-frame poster-flip-face poster-flip-back overflow-hidden rounded-xl border border-zinc-700 bg-zinc-950 p-4",
                                div { class: "flex items-baseline justify-between",
                                    h3 { class: "text-sm font-semibold text-zinc-100", "{name}" }
                                    span { class: "text-[10px] text-zinc-500", "#{rank}" }
                                }
                                p { class: "mt-1 text-[10px] text-zinc-500", "{desc}" }
                                div { class: "mt-3 space-y-2",
                                    for row in dim_rows.iter() {
                                        div { class: "flex items-baseline justify-between border-b border-zinc-800/60 pb-1",
                                            span { class: "text-[11px] text-zinc-500", "{row.label}" }
                                            span { class: "text-[11px] text-zinc-200", "{row.value}" }
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
                            span { class: "min-w-0 truncate text-xs font-medium tracking-wide text-zinc-200", "{name}" }
                            span { class: "ml-auto text-[10px] font-medium italic text-zinc-500", "#{rank}" }
                        }
                    }
                    p { class: "text-[11px] leading-relaxed text-zinc-500", "{desc}" }
                }
            }
        }
    }
}

/// 卡牌倾斜 hover 的通用几何: 鼠标坐标 -> (rotateX, rotateY)。
/// 卡片尺寸硬编码 (RadarFlipCard 460x320, 原实现如此), 仅为演示近似。
fn tilt_from(x: f64, y: f64, w: f64, h: f64, ax: f64, ay: f64) -> (f64, f64) {
    let nx = (x / w - 0.5).clamp(-0.5, 0.5);
    let ny = (y / h - 0.5).clamp(-0.5, 0.5);
    (-ny * ax, nx * ay)
}
