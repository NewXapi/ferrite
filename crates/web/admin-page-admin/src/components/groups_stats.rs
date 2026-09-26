//! 分组概览统计区(编号段 1):五张概览卡。
//!
//! 纯渲染组件:stats 由页面从列表派生后传入,组件内部零状态(无 `use_signal`、
//! 无事件处理)。
//!
//! 边界:统计数值(总数 / 启用 / 停用 / 平均倍率 / 非基准倍率)由 `page.rs` 的
//! 派生块算好,本文件不做任何过滤或聚合;单张概览卡的外观由 `modal.rs` 的
//! `StatCard` 提供,本文件只负责区段头与网格排布。

use dioxus::prelude::*;

use crate::components::groups_modal::StatCard;
use crate::shared::SEC_STATS;

/// 分组概览统计区。
///
/// 【是什么】分组 tab 的编号段 1:区段标题 + 五张概览卡的响应式网格。
///
/// 【做什么】把 `stats` 里的 `(数值, 标签)` 逐条铺成 `StatCard`。不负责数值计算
/// (在 `page.rs`)、不负责卡片刻度与配色(在 `modal.rs` 的 `StatCard`)。
///
/// 【交互逻辑】纯展示,无交互:无 `EventHandler`,无网络请求,不持有任何状态。
///
/// 【样式】外壳 `section#groups-sec-stats` 为 `scroll-mt-8 space-y-3`;标题
/// `text-lg font-medium text-foreground`;网格 `grid grid-cols-1 gap-3
/// md:grid-cols-3 lg:grid-cols-5`(手机 1 / 中屏 3 / 大屏 5 列)。
///
/// 【子组件组成】`StatCard`(五张概览卡,来自 `modal.rs`)。
///
/// 【数据流】
/// - 对内(入):`stats` —— 固定 5 元组的 `(展示数值, 静态标签)`;数值由页面从
///   `groups` 派生(总分组数 / 启用中 / 已停用 / 平均倍率 / 非基准倍率),
///   标签是 `shared.rs` 的 `LBL_STAT_*` 常量。
/// - 对外(出):无。
#[component]
pub fn GroupsStatsSection(stats: Vec<(String, &'static str)>) -> Element {
    rsx! {
        section { id: "groups-sec-stats", class: "scroll-mt-8 space-y-3",
            h2 { class: "{ui::TYPE_TITLE}", "{SEC_STATS}" }
            div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                for (value, label) in stats {
                    StatCard { value, label }
                }
            }
        }
    }
}
