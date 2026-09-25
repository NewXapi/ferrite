//! 别名统计区组件(编号段 1):五张概览卡。
//! 纯渲染组件:stats 由页面从 rows 派生后传入,组件内部零状态。
//!
//! 【是什么】别名 tab 顶部(编号段 1)的五格概览统计条。
//!
//! 【做什么】把页面算好的五项数字渲染成统计卡;不做计数、不做筛选、不拉数据。
//!
//! 【交互逻辑】纯展示,无交互 —— 组件内无按钮、无 `use_signal`、无网络调用。
//!
//! 【样式】外壳 `section#aliases-sec-stats` 带 `scroll-mt-8 space-y-3`(供 ScrollSpy
//! 锚点定位);标题 `text-lg font-medium text-zinc-100`;卡片网格
//! `grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5` —— 手机 1 列、中屏 3 列、
//! 大屏 5 列。
//!
//! 【子组件组成】`crate::tab_page_groups::StatCard`(逐项渲染,本组件不定义卡片样式)。
//!
//! 【数据流】
//! - 对内(入):`stats` 为由页面从 `rows` 派生的 `Vec<(String, &'static str)>`,
//!   每项是「显示值, 标签文案」,顺序固定为 总数 / 标准 1.0× / 自定倍率 / 免费 / 平均倍率。
//! - 对外(出):无 —— 无 EventHandler、无 Signal 写回。

use dioxus::prelude::*;

use crate::components::aliases_stats_section::StatCard;
use crate::shared::SEC_STATS;

#[component]
pub fn AliasesStatsSection(stats: Vec<(String, &'static str)>) -> Element {
    rsx! {
        section { id: "aliases-sec-stats", class: "scroll-mt-8 space-y-3",
            h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
            div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                for (value, label) in stats {
                    StatCard { value, label }
                }
            }
        }
    }
}