//! 渠道卡片网格区(编号段 3):标题计数 + 四态分支 + 新卡示例 + ChannelCard 网格。
//!
//! 纯展示组件:数据以值传入;交互通过 EventHandler 抛回页面。
//! on_toggle 收 (key, target_status) —— 卡片算好目标状态(1=启用 2=停用)抛上来,
//! 页面直接写 API,组件内不持状态。
//!
//! 边界:筛选在 `page.rs` 调 `filter_channels` 完成,本文件不再过滤;启停/删除的
//! 网络写回在页面的 `make_write` 工厂里,本文件只把 `(key, 目标状态)` 转发上去。

use contract::api::admin::ChannelDto;
use dioxus::prelude::*;
use ui::ChannelCard as PrototypeChannelCard;

use super::card::ChannelCard;
use super::shared::{
    BTN_RETRY, LBL_PROTOTYPE_REGION, MSG_EMPTY, MSG_LOAD_FAILED, MSG_LOADING_LIST,
    OPT_BADGE_LOADING, SEC_LIST,
};

/// 渠道卡片网格区:错误 / 加载 / 空 / 网格 四态。
///
/// 【是什么】渠道 tab 的编号段 3:标题行 + 计数徽标 + 四态分支 + 新卡示例 + 渠道卡网格。
///
/// 【做什么】按 `err` / `loading` / `filtered` 渲染四种形态之一;有数据时先铺一张
/// ui-components 的原型卡示例,再逐条渲染真实 `ChannelCard`。不负责筛选、不拉数据、
/// 不做启停/删除请求(只把意图抛回页面)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 错误态「重试」→ `on_retry`(MouseEvent)抛回页面 `reload + 1`。
/// - 卡片「编辑」→ 包装成 `on_edit.call(edit_key)`(String),页面 `open_edit` 回填表单。
/// - 卡片「启用/停用」→ 组件先算目标状态(`status == 1` 时目标 2,否则 1),
///   调 `on_toggle.call((key, target))`,由页面落成 `WriteOp::Toggle` 发 API。
/// - 卡片「删除」→ `on_delete.call(key)`,由页面落成 `WriteOp::Delete` 发 API。
/// 数据交互:本组件自身**不发任何网络请求**。
///
/// 【样式】外壳 `section#channels-sec-list` 为 `scroll-mt-8 space-y-4`;标题
/// `text-lg font-medium text-zinc-100` + 右侧 `rounded-full bg-zinc-800` 计数胶囊;
/// 错误态红底 `border-red-800/60 bg-red-950/40`,加载/空态为虚线描边
/// `border-dashed border-zinc-700 bg-zinc-900/50 py-16`;新卡示例区 `mb-4` 且带
/// `role="region" aria-label="新卡示例"`;两处网格均为 `grid grid-cols-1 gap-3
/// md:grid-cols-3 lg:grid-cols-5`(手机 1 / 中屏 3 / 大屏 5 列)。
///
/// 【子组件组成】`ui::ChannelCard`(别名为 `PrototypeChannelCard`,只渲染
/// `filtered.first()` 作为新卡示例)、`super::card::ChannelCard`(真实卡片)。
///
/// 【数据流】
/// - 对内(入):`loading`(首屏加载态;后台刷新走页面的 `refreshing`,不传这里)、
///   `err`(优先于 loading 渲染)、`filtered`(页面已筛好的渠道列表)、
///   `on_edit` / `on_toggle` / `on_delete` / `on_retry` 四个回调。
/// - 对外(出):`on_edit(String)` → 页面 `open_edit` 回填并开弹窗;`on_toggle((String, i16))`
///   → 页面 `on_toggle_card` → `set_channel_status_api`;`on_delete(String)` → 页面
///   `on_delete_card` → `delete_channel_api`;`on_retry(MouseEvent)` → 页面重拉。
#[component]
pub fn ChannelsListSection(
    /// 列表加载中(骨架态;后台刷新不清列表,走 refreshing,不传这里)
    loading: bool,
    /// 拉取失败摘要(错误态;Some 时优先于 loading 渲染)
    err: Option<String>,
    /// 筛选后的渠道列表,页面派生
    filtered: Vec<ChannelDto>,
    /// 请求编辑(开弹窗回填)
    on_edit: EventHandler<String>,
    /// 启停切换:(key, 目标状态 1|2)
    on_toggle: EventHandler<(String, i16)>,
    /// 请求删除
    on_delete: EventHandler<String>,
    /// 错误态「重试」
    on_retry: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        section { id: "channels-sec-list", class: "scroll-mt-8 space-y-4",
            div { class: "flex items-center justify-between",
                h2 { class: "text-lg font-medium text-zinc-100", "{SEC_LIST}" }
                span { class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400",
                    if loading { "{OPT_BADGE_LOADING}" } else { "{filtered.len()} 个渠道" }
                }
            }

            if let Some(e) = err {
                div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 py-10 text-center",
                    p { class: "text-sm text-red-300", "{MSG_LOAD_FAILED}" }
                    p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                    button {
                        class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                        onclick: on_retry,
                        "{BTN_RETRY}"
                    }
                }
            } else if loading {
                div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "{MSG_LOADING_LIST}" }
                }
            } else if filtered.is_empty() {
                div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "{MSG_EMPTY}" }
                }
            } else {
                if let Some(channel) = filtered.first().cloned() {
                    {
                        rsx! {
                            div {
                                class: "mb-4 grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                                role: "region",
                                "aria-label": "{LBL_PROTOTYPE_REGION}",
                                "data-testid": "channel-card-prototype",
                                PrototypeChannelCard {
                                    channel,
                                }
                            }
                        }
                    }
                }
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                    "data-testid": "channels-list",
                    for c in filtered {
                        {
                            let edit_key = c.key.clone();
                            let toggle_key = c.key.clone();
                            let delete_key = c.key.clone();
                            // 后端 status 语义: 1=启用 2=停用 (channels status 校验 [1,2])
                            let target = if c.status == 1 { 2 } else { 1 };
                            rsx! {
                                ChannelCard {
                                    key: "{c.key}",
                                    channel: c,
                                    on_edit: move |_| on_edit.call(edit_key.clone()),
                                    on_toggle: move |_| on_toggle.call((toggle_key.clone(), target)),
                                    on_delete: move |_| on_delete.call(delete_key.clone()),
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
