//! 别名统计区(编号段 1):五张概览卡。
//! 纯渲染组件:stats 由页面从 rows 派生后传入,组件内部零状态。

use dioxus::prelude::*;

use super::shared::SEC_STATS;
use crate::tab_page_groups::StatCard;

/// 别名概览统计区:总别名 / 标准 1.0× / 自定倍率 / 免费 / 平均倍率 五张卡。
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
