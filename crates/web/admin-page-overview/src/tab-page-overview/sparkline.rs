//! 统计卡底部的 12 点迷你面积线(无图表库,手绘 SVG)。

use dioxus::prelude::*;

use crate::api;

/// 统计卡底部的 12 点手绘迷你面积线（无图表库；配方见 deepdive §二-3：
/// 160x36 viewBox + preserveAspectRatio="none"、min-max 归一化、line+area 双 path、
/// area 用 currentColor 线性渐变 0.24→0、non-scaling-stroke 保证任意拉伸线宽恒定）。
///
/// 数据不足两点（空序列 / 单点）→ 渲染等高占位防布局跳动（诚实降级）。
#[component]
pub fn Sparkline(series: Vec<f64>, gradient_id: &'static str) -> Element {
    let Some((line_d, area_d)) = api::sparkline_svg_paths(&series, 160.0, 36.0) else {
        return rsx! {
            div {
                class: "pointer-events-none mt-2 h-9 rounded-md border border-dashed border-border/60 bg-accent/30",
                aria_hidden: "true",
            }
        };
    };
    rsx! {
        svg {
            class: "pointer-events-none mt-2 h-9 w-full text-muted-foreground",
            view_box: "0 0 160 36",
            preserve_aspect_ratio: "none",
            "aria-hidden": "true",
            "data-testid": "{gradient_id}",
            defs {
                linearGradient {
                    id: "{gradient_id}",
                    x1: "0",
                    y1: "0",
                    x2: "0",
                    y2: "1",
                    stop { offset: "0", stop_color: "currentColor", stop_opacity: "0.24" }
                    stop { offset: "1", stop_color: "currentColor", stop_opacity: "0" }
                }
            }
            path { d: "{area_d}", fill: "url(#{gradient_id})" }
            path {
                d: "{line_d}",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2.25",
                vector_effect: "non-scaling-stroke",
                stroke_linecap: "round",
                stroke_linejoin: "round",
            }
        }
    }
}
