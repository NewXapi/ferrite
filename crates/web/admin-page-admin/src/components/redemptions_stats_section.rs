//! 兑换码概览统计区(编号段 1):五张概览卡。
//!
//! 纯渲染组件:stats 由页面从列表派生后传入,组件内部零状态(无 `use_signal`、
//! 无事件处理)。
//!
//! 边界:统计数值(总数 / 未使用 / 已核销 / 已停用 / 可用面额)由 `page.rs` 的派生块
//! 算好,本文件不做任何过滤或聚合;单张概览卡的外观复用 `tab-page-groups` 的
//! `StatCard`,本文件只负责区段头与网格排布。

use dioxus::prelude::*;

use crate::components::groups_modal::StatCard;
use crate::shared::SEC_STATS_REDEMPTIONS;

/// 兑换码概览统计区。
///
/// 【是什么】兑换码 tab 的编号段 1:区段标题 + 五张概览卡的响应式网格。
///
/// 【做什么】把 `stats` 里的 `(数值, 标签)` 逐条铺成 `StatCard`。不负责数值计算
/// (在 `page.rs`)、不负责卡片刻度与配色(复用 groups 的 `StatCard`)。
///
/// 【交互逻辑】纯展示,无交互:无 `EventHandler`,无网络请求,不持有任何状态。
///
/// 【样式】外壳 `section#reds-sec-stats` 为 `scroll-mt-8 space-y-3`,并带
/// `data-testid="redemptions-stats"`、`role="region"`、`aria-label=SEC_STATS_REDEMPTIONS`;
/// 标题 `{ui::TYPE_TITLE}`;网格 `grid grid-cols-1 gap-3
/// md:grid-cols-3 lg:grid-cols-5`(手机 1 / 中屏 3 / 大屏 5 列)。
///
/// 【子组件组成】`StatCard`(五张概览卡,来自 `tab-page-groups::modal`)。
///
/// 【数据流】
/// - 对内(入):`stats` —— 固定 5 元组的 `(展示数值, 静态标签)`;数值由页面从
///   `reds` 派生(总数 / 未使用 / 已核销 / 已停用 / 可用面额合计),
///   标签是 `shared.rs` 的 `LBL_STAT_*` 常量。
/// - 对外(出):无。
#[component]
pub fn RedemptionsStatsSection(stats: Vec<(String, &'static str)>) -> Element {
    rsx! {
        section {
            id: "reds-sec-stats",
            "data-testid": "redemptions-stats",
            role: "region",
            "aria-label": SEC_STATS_REDEMPTIONS,
            class: "scroll-mt-8 space-y-3",
            h2 { class: "{ui::TYPE_TITLE}", "{SEC_STATS_REDEMPTIONS}" }
            div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                for (value, label) in stats {
                    StatCard { value, label }
                }
            }
        }
    }
}
