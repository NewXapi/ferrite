//! 渠道概览统计区(编号段 1):五张概览卡。
//! 纯渲染组件:stats 由页面从列表派生后传入,组件内部零状态。

use dioxus::prelude::*;

use crate::tab_page_groups::StatCard;

/// 渠道概览统计区:总数 / 启用 / 停用 / 密钥 / 分组 五张卡。
#[component]
pub fn ChannelsStatsSection(stats: Vec<(String, &'static str)>) -> Element {
    rsx! {
        section { id: "channels-sec-stats", class: "scroll-mt-8 space-y-3",
            h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
            div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                for (value, label) in stats {
                    StatCard { value, label }
                }
            }
        }
    }
}

const SEC_STATS: &str = "渠道概览";
