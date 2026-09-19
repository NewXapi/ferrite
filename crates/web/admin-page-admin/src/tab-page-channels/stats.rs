//! 渠道概览统计区(编号段 1):五张概览卡。
//! 纯渲染组件:stats 由页面从列表派生后传入,组件内部零状态。
//!
//! 边界:只负责统计区的排版,计数全部在 `page.rs` 的派生块里完成(启用 / 停用
//! 按 `status == 1`、密钥数按 `key_count` 求和、分组数按去重集合大小);
//! 卡片外观由 `tab_page_groups::StatCard` 决定,本文件不复写样式。

use dioxus::prelude::*;

use super::shared::SEC_STATS;
use crate::tab_page_groups::StatCard;

/// 渠道概览统计区:总数 / 启用 / 停用 / 密钥 / 分组 五张卡。
///
/// 【是什么】渠道 tab 顶部(编号段 1)的五格概览统计条。
///
/// 【做什么】把页面算好的五项数字铺成统计卡;不做计数、不做筛选、不拉数据。
///
/// 【交互逻辑】纯展示,无交互 —— 无按钮、无 `use_signal`、无网络调用。
///
/// 【样式】外壳 `section#channels-sec-stats` 带 `scroll-mt-8 space-y-3`(供
/// ScrollSpy 锚点定位);标题 `text-lg font-medium text-zinc-100`;卡片网格
/// `grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5` —— 手机 1 列、中屏 3 列、
/// 大屏 5 列。
///
/// 【子组件组成】`tab_page_groups::StatCard`(逐项渲染,本组件不定义卡片样式)。
///
/// 【数据流】
/// - 对内(入):`stats` 为由 `page.rs` 从渠道列表派生的 `Vec<(String, &'static str)>`,
///   每项是「显示值, 标签文案」,顺序固定为 总渠道数 / 正常启用 / 停用·异常 /
///   密钥总数 / 绑定分组数。
/// - 对外(出):无 —— 无 EventHandler、无 Signal 写回。
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
