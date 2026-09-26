//! 统计区组件:总用户 / 启用中 / 本月新增 / 总发放额度 / 总消耗 五张统计卡。
//! 从 tab-page/users.rs 段 1 沉降(R1.1 粒度下限 = 一个编号段落)。

use dioxus::prelude::*;
use ui::StatCard;

use crate::shared::SEC_STATS;

/// 统计区(页面段落 1)。
///
/// 【是什么】用户概览:五张 StatCard——总用户 / 启用中 / 本月新增 / 总发放额度 / 总消耗。
///
/// 【做什么】只负责把页面算好的统计值渲染成卡片网格;不负责拉取(页面层 reload
/// effect)、不负责筛选,也不做任何网络请求。
///
/// 【交互逻辑】纯展示,无交互。
///
/// 【样式】`section#users-sec-stats.scroll-mt-8.space-y-3` + h2 标题;卡片网格
/// 手机 1 栏 / 平板 3 栏 / Web 5 栏,每卡各占 1 栏(class 与原页面逐字一致)。
///
/// 【子组件组成】`ui::StatCard` ×5。
///
/// 【数据流】
/// - 对内(入):`stats`——页面层从用户列表派生的 (值, 标签) 数组(与筛选/列表
///   共用同一次 `users()` 求值,故派生留在页面层)。
/// - 对外(出):无。
#[component]
pub fn UsersStatsSection(stats: [(String, &'static str); 5]) -> Element {
    rsx! {
        section { id: "users-sec-stats", class: "scroll-mt-8 space-y-3",
            h2 { class: "{ui::TYPE_TITLE}", "{SEC_STATS}" }
            // 宽度约定:手机 1 栏 / 平板 3 栏 / Web 5 栏,每卡各占 1 栏。
            div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                for (value, label) in stats {
                    StatCard { value, label }
                }
            }
        }
    }
}
