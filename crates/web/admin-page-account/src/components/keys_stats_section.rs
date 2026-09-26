//! keys 面板统计区段组件:「个人数据」区的 5 张卡片（rust-ui Card 重构）。
//!
//! 纯展示组件——数据全部由页面层经 props 传入,本组件不发请求、不改状态。

use dioxus::prelude::*;

use ui::components::rui_card::{Card, CardContent};

use crate::usage_support::fmt_quota;

/// 区段标题文案。
const SEC_STATS: &str = "个人数据";

/// 【是什么】统计卡片区段组件，展示 5 项关键指标。
///
/// 【做什么】负责渲染「个人数据」区的 5 张卡片：密钥总数、近30天消耗、近30天请求、剩余额度、成功率。不负责数据获取，仅做纯展示。
///
/// 【交互逻辑】纯展示，无交互。
///
/// 【样式】网格布局：移动端 1 列、平板 3 列、桌面 5 列，间距 gap-3。每张卡片为 rust-ui Card（CardContent 放数值与标签）。
///
/// 【子组件组成】rui Card + CardContent × 5
///
/// 【数据流】通过 props 接收：keys_loaded (Signal<bool>)、keys_len (usize)、remaining (Option<i64>)、pending、none_v 占位文案。无输出。
#[component]
pub fn KeysStatsSection(
    keys_loaded: Signal<bool>,
    keys_len: usize,
    remaining: Option<i64>,
    pending: String,
    none_v: String,
) -> Element {
    rsx! {
        section {
            id: "keys-sec-stats",
            class: "scroll-mt-8 space-y-3",
            h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
            div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                Card { class: "px-4 py-4",
                    CardContent { class: "gap-1",
                        div { class: "text-2xl font-semibold text-zinc-100",
                            if keys_loaded() { "{keys_len}" } else { "{pending}" }
                        }
                        div { class: "text-xs text-zinc-500", "密钥总数" }
                    }
                }
                Card { class: "px-4 py-4",
                    CardContent { class: "gap-1",
                        div { class: "text-2xl font-semibold text-zinc-100", "{none_v}" }
                        div { class: "text-xs text-zinc-500", "近 30 天消耗 (暂无数据)" }
                    }
                }
                Card { class: "px-4 py-4",
                    CardContent { class: "gap-1",
                        div { class: "text-2xl font-semibold text-zinc-100", "{none_v}" }
                        div { class: "text-xs text-zinc-500", "近 30 天请求 (暂无数据)" }
                    }
                }
                Card { class: "px-4 py-4",
                    CardContent { class: "gap-1",
                        div { class: "text-2xl font-semibold text-zinc-100",
                            {remaining.map(fmt_quota).unwrap_or_else(|| pending.clone())}
                        }
                        div { class: "text-xs text-zinc-500", "剩余额度 (≈$)" }
                    }
                }
                Card { class: "px-4 py-4",
                    CardContent { class: "gap-1",
                        div { class: "text-2xl font-semibold text-zinc-100", "{none_v}" }
                        div { class: "text-xs text-zinc-500", "成功率 (暂无数据)" }
                    }
                }
            }
        }
    }
}
