//! 分组概览统计区(编号段 1):五张概览卡。
//! 纯渲染组件:stats 由页面从列表派生后传入,组件内部零状态。

use dioxus::prelude::*;

use super::shared::SEC_STATS;
use crate::tab_page_groups::StatCard;

/// 分组概览统计区。
#[component]
pub fn GroupsStatsSection(stats: Vec<(String, &'static str)>) -> Element {
    rsx! {
        section { id: "groups-sec-stats", class: "scroll-mt-8 space-y-3",
            h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
            div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                for (value, label) in stats {
                    StatCard { value, label }
                }
            }
        }
    }
}
