//! 渠道卡片网格区(编号段 3):标题计数 + 四态分支 + 新卡示例 + ChannelCard 网格。
//!
//! 纯展示组件:数据以值传入;交互通过 EventHandler 抛回页面。
//! on_toggle 收 (key, target_status) —— 卡片算好目标状态(1=启用 2=停用)抛上来,
//! 页面直接写 API,组件内不持状态。

use contract::api::admin::ChannelDto;
use dioxus::prelude::*;
use ui::ChannelCard as PrototypeChannelCard;

use super::card::ChannelCard;

const SEC_LIST: &str = "渠道列表";

/// 渠道卡片网格区:错误 / 加载 / 空 / 网格 四态。
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
                    if loading { "加载中…" } else { "{filtered.len()} 个渠道" }
                }
            }

            if let Some(e) = err {
                div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 py-10 text-center",
                    p { class: "text-sm text-red-300", "加载渠道失败" }
                    p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                    button {
                        class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                        onclick: on_retry,
                        "重试"
                    }
                }
            } else if loading {
                div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "正在加载渠道…" }
                }
            } else if filtered.is_empty() {
                div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "没有匹配的渠道" }
                }
            } else {
                if let Some(channel) = filtered.first().cloned() {
                    {
                        rsx! {
                            div {
                                class: "mb-4 grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                                role: "region",
                                "aria-label": "新卡示例",
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
