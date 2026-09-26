//! 真实用量榜工具条:标题行 + 时间窗切换胶囊。
//!
//! - 是什么:真实用量榜区顶部的标题行(8090 预览反馈④:时间窗 tab 与标题同行)。
//! - 负责什么:渲染标题、口径副标题与档位切换器;不取数、不判空。
//! - 交互逻辑:`timeframe` 由页面持有并以 Signal 双向绑定 —— 切档即写回,页面
//!   `use_effect` 依赖它重拉榜单与升降速;组件内部零 `use_signal`。
//! - 样式:`flex-wrap items-center justify-between` + 底边 `border-b border-zinc-800/80 pb-4`;
//!   切换器复用跨 tab 的 [`TimeframeTabs`](crate::shared::TimeframeTabs)。
//! - 数据流通:入参 `model_count` 为窗口内有调用的模型数(页面从 rows 派生)。

use dioxus::prelude::*;

use super::shared::{
    TESTID_TIMEFRAME_PREFIX, USAGE_SUBTITLE_HEAD, USAGE_SUBTITLE_TAIL, USAGE_TITLE,
};
use crate::shared::TimeframeTabs;

/// 真实用量榜工具条。
#[component]
pub fn UsageToolbar(
    /// 当前时间窗档位(页面持有,供取数复用)。
    timeframe: Signal<&'static str>,
    /// 窗口内有调用的模型数(副标题口径)。
    model_count: usize,
) -> Element {
    rsx! {
        div { class: "flex flex-wrap items-center justify-between gap-3 border-b border-zinc-800/80 pb-4",
            div {
                h2 { class: "text-lg font-bold tracking-tight text-zinc-100 md:text-xl", "{USAGE_TITLE}" }
                p { class: "mt-1 text-xs text-zinc-400", "{USAGE_SUBTITLE_HEAD}{model_count}{USAGE_SUBTITLE_TAIL}" }
            }
            TimeframeTabs { timeframe, testid_prefix: TESTID_TIMEFRAME_PREFIX }
        }
    }
}
