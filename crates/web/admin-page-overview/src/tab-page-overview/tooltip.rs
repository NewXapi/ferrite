//! 趋势图悬浮卡:通用外框 + 两种卡体(整列分解 / 单色块详情)。
//!
//! - 是什么:`TrendTooltipContainer` 是外框(阴影/圆角/背景/内边距/定位),
//!   `TrendTipCard` 是按 `TrendTip` 二选一渲染卡体的分发组件。
//! - 数据流通:悬浮内容由直方图在鼠标事件里构造 `TrendTip` 写回面板 signal;
//!   本文件只消费,不持有状态、不发请求。
//! - 样式:`fixed` 定位(不被滚动容器裁剪)+ `bg-card/95 backdrop-blur-md`;
//!   窄屏(`max-sm`)改为贴边底部条,避免溢出。

use dioxus::prelude::*;

use super::shared::{TREND_TIP_TOTAL, TrendTip};
use crate::shared::fmt_raw;

/// 悬浮卡通用外框组件,保持单色块和整列的阴影、圆角、背景和内边距完全统一。
///
/// - 智能定位:① 水平翻转 —— `x > 260px` 时往左侧展开,否则往右侧展开,防止
///   手机与窄屏边缘被右边框裁切;② 垂直修正 —— `y` 下限 80px,避免被顶部导航遮挡。
/// - 初始态(0,0)以 `opacity-0 scale-95` 隐藏,避免首帧在左上角闪现。
#[component]
pub fn TrendTooltipContainer(x: f64, y: f64, label: String, children: Element) -> Element {
    let transform = if x > 260.0 {
        "translate(calc(-100% - 12px), -50%)"
    } else {
        "translate(12px, -50%)"
    };
    let clamped_y = y.max(80.0);
    let opacity_class = if x == 0.0 && y == 0.0 {
        "opacity-0 scale-95"
    } else {
        "opacity-100 scale-100"
    };
    rsx! {
        div {
            class: "pointer-events-none fixed z-50 rounded-xl border border-border/80 bg-card/95 p-3 text-xs shadow-2xl backdrop-blur-md transition-all duration-150 ease-out {opacity_class} max-sm:left-3! max-sm:right-3! max-sm:bottom-4! max-sm:top-auto! max-sm:transform-none! max-sm:w-auto!",
            style: "left: {x}px; top: {clamped_y}px; transform: {transform}; max-width: calc(100vw - 24px);",
            p { class: "mb-1.5 text-xs font-semibold text-muted-foreground", "{label}" }
            {children}
        }
    }
}

/// 趋势悬浮卡分发:整列模式列全部模型明细 + Total,色块模式列单模型一行。
///
/// 由面板在 `tip()` 为 `Some` 时渲染;两种模式共用 [`TrendTooltipContainer`] 外框。
#[component]
pub fn TrendTipCard(tip: TrendTip) -> Element {
    match tip {
        TrendTip::Column(x, y, label, rows, total) => rsx! {
            TrendTooltipContainer { x, y, label,
                div { class: "mb-2.5 flex items-center justify-between border-b border-zinc-800/80 pb-2 text-xs text-zinc-400",
                    span { "{TREND_TIP_TOTAL}" }
                    span { class: "font-mono font-semibold text-zinc-100", "{fmt_raw(total as i64)}" }
                }
                div { class: "flex flex-col gap-1.5",
                    for (name, color, v) in rows.iter() {
                        div { class: "flex items-center justify-between gap-4 text-xs",
                            div { class: "flex items-center min-w-0",
                                span { class: "h-2 w-2 shrink-0 rounded-[2px]", style: "background: {color}" }
                                span { class: "truncate text-zinc-300 ml-2", "{name}" }
                            }
                            span { class: "shrink-0 font-mono font-medium text-zinc-100", "{fmt_raw(*v as i64)}" }
                        }
                    }
                }
            }
        },
        TrendTip::Segment(x, y, label, name, color, v) => rsx! {
            TrendTooltipContainer { x, y, label,
                div { class: "flex items-center justify-between gap-4 text-xs",
                    div { class: "flex items-center min-w-0",
                        span { class: "h-2.5 w-2.5 shrink-0 rounded-[2px]", style: "background: {color}" }
                        span { class: "font-medium text-zinc-200 ml-2", "{name}" }
                    }
                    span { class: "shrink-0 font-mono font-bold text-zinc-100", "{fmt_raw(v as i64)}" }
                }
            }
        },
    }
}
