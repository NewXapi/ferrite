//! keys 面板统计区段组件:「个人数据」区的 5 张卡片（rust-ui Card 重构）。
//!
//! 纯展示组件——数据全部由页面层经 props 传入,本组件不发请求、不改状态。

use dioxus::prelude::*;

use ui::components::rui_card::{Card, CardSize};

use crate::usage_support::fmt_quota;

/// 区段标题文案。
const SEC_STATS: &str = "个人数据";

/// 【是什么】统计卡片区段组件，展示 5 项关键指标。
///
/// 【做什么】负责渲染「个人数据」区的 5 张卡片：密钥总数、近30天消耗、近30天请求、剩余额度、成功率。不负责数据获取，仅做纯展示。
///
/// 【交互逻辑】纯展示，无交互。
///
/// 【样式】网格布局：移动端 1 列、平板 3 列、桌面 5 列，间距 gap-3。卡片为 rust-ui Card（Sm 档 = 原 stat tile 的 px-4 py-3），值/标签排版逐字沿用原版。
///
/// 【子组件组成】rui Card × 5
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
                for tile in [
                    (if keys_loaded() { keys_len.to_string() } else { pending.clone() }, "密钥总数"),
                    (none_v.clone(), "近 30 天消耗 (暂无数据)"),
                    (none_v.clone(), "近 30 天请求 (暂无数据)"),
                    (remaining.map(fmt_quota).unwrap_or_else(|| pending.clone()), "剩余额度 (≈$)"),
                    (none_v.clone(), "成功率 (暂无数据)"),
                ] {
                    Card { size: CardSize::Sm,
                        p { class: "text-xl font-semibold tracking-tight text-white", "{tile.0}" }
                        p { class: "mt-0.5 text-xs text-zinc-500", "{tile.1}" }
                    }
                }
            }
        }
    }
}
