//! 用量趋势直方图:横向虚网格 + 按模型堆叠柱 + 两级悬浮事件 + X 轴标签。
//!
//! - 是什么:趋势面板左栏(`xl:col-span-2`),一柱 = 一个时间桶,柱内按模型堆叠。
//! - 负责什么:柱高换算、Y 轴刻度、柱/段的鼠标事件与悬浮卡数据整形;不取数。
//! - 交互逻辑:悬停柱顶透明区 → 整列明细(`TrendTip::Column`);悬停有色段 →
//!   单模型详情(`TrendTip::Segment`,事件 `stop_propagation` 防冒泡到整列);
//!   移出图表区清空悬浮。悬浮状态由面板持有并以 Signal 传入,本组件零 `use_signal`。
//! - 样式:柱间距 `gap: 3px`、堆叠段连续无间隙(subpixel 间隙旧版视觉不一致);
//!   悬停整列时通顶半透明光柱 `group-hover:bg-zinc-100/10`,有色段 `hover:brightness-125`。
//! - 数据流通:入参 `buckets`/`names` 来自面板已 pivot 的窗口数据;出参只有
//!   `tip` signal(就地写入)。柱明细的排序与折叠收在 `api::trend_column_tip`。

use dioxus::prelude::*;

use super::shared::TrendTip;
use crate::api::{self, TrendBucketFE};
use crate::shared::{MODEL_COLORS, fmt_raw};

/// 堆叠直方图(含 Y 轴虚网格与 X 轴标签)。
#[component]
pub fn TrendHistogram(
    /// 已 pivot 的时间桶(顺序即 X 轴顺序)。
    buckets: Vec<TrendBucketFE>,
    /// 模型名序列,与桶内 `per_model` 下标一一对应(决定段色与名称)。
    names: Vec<String>,
    /// Y 轴封顶值(由面板用 `api::nice_axis_max` 算出)。
    axis_max: f64,
    /// 悬浮卡状态(面板持有;本组件只写入)。
    tip: Signal<Option<TrendTip>>,
) -> Element {
    rsx! {
        div {
            class: "xl:col-span-2",
            onmouseleave: move |_| tip.set(None),
            div { class: "relative",
                div { class: "pointer-events-none absolute inset-0 flex flex-col justify-between py-0", aria_hidden: "true",
                    // 顶格是封顶线 (axis_max), 其下三条是 step 等分, 最后是 0 基线
                    for i in [4, 3, 2, 1] {
                        div { class: "relative w-full border-t border-dashed border-zinc-800",
                            span { class: "absolute -top-2 right-0 text-[10px] text-zinc-600", "{fmt_raw((axis_max * i as f64 / 4.0) as i64)}" }
                        }
                    }
                    div { class: "relative w-full border-t border-dashed border-zinc-800",
                        span { class: "absolute -top-2 right-0 text-[10px] text-zinc-600", "0" }
                    }
                }
                div { class: "relative flex h-56 items-end", style: "gap: 3px",
                    for b in buckets.iter() {
                        {
                            let hpct = (b.total / axis_max * 100.0).max(3.0);
                            let label = b.label.clone();
                            // 列模式明细: 排序(值降序) + Total + 超 10 行折叠「+N more」
                            // —— 纯整形逻辑收在 api::trend_column_tip(可单测),渲染层只消费结果
                            let col_tip = api::trend_column_tip(&b.per_model, &names, &MODEL_COLORS);
                            let col_rows = col_tip.rows;
                            let col_total = col_tip.total;
                            rsx! {
                                div {
                                    class: "group relative flex h-full flex-1 cursor-default flex-col justify-end",
                                    onmouseleave: move |_| tip.set(None),
                                    // 悬停整列: 通顶全高半透明背景光柱
                                    div { class: "pointer-events-none absolute inset-x-0 top-0 bottom-0 rounded-sm transition-colors duration-150 group-hover:bg-zinc-100/10" }

                                    // 顶层无色透明区: 悬停时显示该列全部明细
                                    div {
                                        class: "pointer-events-auto flex-1 w-full cursor-pointer",
                                        onmouseenter: {
                                            let c_label = label.clone();
                                            let c_rows = col_rows.clone();
                                            move |evt| {
                                                let p = evt.data.client_coordinates();
                                                tip.set(Some(TrendTip::Column(p.x, p.y, c_label.clone(), c_rows.clone(), col_total)));
                                            }
                                        },
                                        onmousemove: {
                                            let c_label = label.clone();
                                            let c_rows = col_rows.clone();
                                            move |evt| {
                                                let p = evt.data.client_coordinates();
                                                tip.set(Some(TrendTip::Column(p.x, p.y, c_label.clone(), c_rows.clone(), col_total)));
                                            }
                                        },
                                    }

                                    // 直方图有色堆叠容器 (堆叠段连续无间隙, 参照 new-api 堆叠图;
                                    // 之前 gap 1.5px 让亚像素小片段看起来像"间隙", 视觉不一致)
                                    div {
                                        class: "relative flex w-full flex-col-reverse overflow-hidden rounded-[3px] transition-all duration-200",
                                        style: "height: {hpct:.1}%",
                                        for (i, v) in b.per_model.iter().enumerate() {
                                            {
                                                let seg_label_1 = b.label.clone();
                                                let seg_name_1 = names[i].clone();
                                                let seg_label_2 = b.label.clone();
                                                let seg_name_2 = names[i].clone();
                                                let seg_color = MODEL_COLORS[i % MODEL_COLORS.len()];
                                                let seg_val = *v;
                                                let denom = if b.total > 0.0 { b.total } else { 1.0 };
                                                rsx! {
                                                    div {
                                                        class: "pointer-events-auto w-full cursor-pointer hover:brightness-125 transition-all",
                                                        style: "height: {(v / denom * 100.0):.1}%; background: {seg_color}",
                                                        onmouseenter: move |evt| {
                                                            evt.stop_propagation();
                                                            let p = evt.data.client_coordinates();
                                                            tip.set(Some(TrendTip::Segment(p.x, p.y, seg_label_1.clone(), seg_name_1.clone(), seg_color, seg_val)));
                                                        },
                                                        onmousemove: move |evt| {
                                                            evt.stop_propagation();
                                                            let p = evt.data.client_coordinates();
                                                            tip.set(Some(TrendTip::Segment(p.x, p.y, seg_label_2.clone(), seg_name_2.clone(), seg_color, seg_val)));
                                                        },
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
            }
            div { class: "mt-2 flex text-[10px] text-zinc-600", style: "gap: 3px",
                for b in buckets.iter() {
                    span { class: "flex-1 truncate text-center",
                        if b.show_label { "{b.label}" }
                    }
                }
            }
        }
    }
}
