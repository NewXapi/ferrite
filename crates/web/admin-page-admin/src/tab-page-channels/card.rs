//! 单个渠道卡片(对齐 GroupCard / UserCard 规范):状态徽标 + 指标行 + 三键操作。
//! 纯展示组件:写回回调以 `EventHandler<()>` 抛回 page。

use dioxus::prelude::*;

use contract::api::admin::ChannelDto;

use crate::tab_page_groups::Badge;
/// 单个渠道卡片 (对齐 GroupCard / UserCard 规范)
#[component]
pub fn ChannelCard(
    channel: ChannelDto,
    on_edit: EventHandler<()>,
    on_toggle: EventHandler<()>,
    on_delete: EventHandler<()>,
) -> Element {
    let initial = channel
        .name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();

    let is_enabled = channel.status == 1;

    let (status_text, status_tone) = if is_enabled {
        (
            "启用中",
            "border-emerald-500/30 bg-emerald-500/20 text-emerald-400",
        )
    } else {
        ("已停用", "border-zinc-700 bg-zinc-800/80 text-zinc-400")
    };

    rsx! {
        div { class: "group flex flex-col justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            "data-testid": "channel-card",
            div { class: "space-y-3",
                // 头部
                div { class: "flex items-start gap-3",
                    div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-zinc-700 bg-zinc-800 text-sm font-semibold text-zinc-200 group-hover:border-zinc-500 transition-colors",
                        "{initial}"
                    }
                    div { class: "min-w-0 flex-1",
                        div { class: "flex items-center justify-between gap-2",
                            h3 { class: "truncate text-sm font-medium text-zinc-100", "{channel.name}" }
                            // UUID 全串不可断:允许收缩并截断,悬停 title 看全值,避免凸出卡片
                            span { class: "min-w-0 max-w-[140px] truncate rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] font-mono text-zinc-400 border border-zinc-700/60",
                                title: "{channel.key}",
                                "#{channel.key}"
                            }
                        }
                        p { class: "mt-0.5 truncate text-[11px] text-zinc-400", "{channel.channel_type} · {channel.groups.join(\", \")}" }
                    }
                }

                // 徽标行
                div { class: "flex flex-wrap gap-1.5",
                    Badge { text: status_text.to_string(), tone: status_tone }
                    Badge { text: channel.groups.join(", "), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-300" }
                    Badge { text: format!("{} 密钥", channel.key_count), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-500" }
                }

                // 指标详情行
                div { class: "space-y-1.5 text-xs pt-1",
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "接口地址" }
                        span { class: "truncate font-mono text-zinc-300 max-w-[140px]", title: "{channel.base_url}", "{channel.base_url}" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "权重" }
                        span { class: "font-medium text-zinc-200", "{channel.weight}" }
                    }
                    if !channel.remark.is_empty() {
                        div { class: "flex justify-between gap-2",
                            span { class: "shrink-0 text-zinc-400", "备注" }
                            span { class: "truncate font-medium text-zinc-200", "{channel.remark}" }
                        }
                    }
                }
            }

            // 底部操作区 (标准三键布局: [编辑] [启用/停用] [删除])
            div { class: "mt-4 flex gap-1.5 border-t border-zinc-800 pt-3",
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white",
                    onclick: move |_| on_edit.call(()),
                    "data-testid": "edit-channel",
                    "编辑"
                }
                button {
                    class: if is_enabled {
                        "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-amber-400 transition-colors hover:bg-zinc-700 hover:text-amber-300"
                    } else {
                        "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-emerald-400 transition-colors hover:bg-zinc-700 hover:text-emerald-300"
                    },
                    onclick: move |_| on_toggle.call(()),
                    "data-testid": "toggle-channel",
                    if is_enabled { "停用" } else { "启用" }
                }
                button {
                    class: "w-7 rounded-lg border border-zinc-800 bg-zinc-800/40 py-1.5 text-xs text-zinc-500 hover:text-red-400 hover:border-red-900/60 transition-colors flex items-center justify-center",
                    title: "删除渠道",
                    "data-testid": "delete-channel",
                    onclick: move |_| on_delete.call(()),
                    "✕"
                }
            }
        }
    }
}
