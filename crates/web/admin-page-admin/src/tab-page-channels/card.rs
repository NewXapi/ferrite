//! 单个渠道卡片(对齐 GroupCard / UserCard 规范):状态徽标 + 指标行 + 三键操作。
//! 纯展示组件:写回回调以 `EventHandler<()>` 抛回 page。
//!
//! 边界:卡片不持任何列表状态、不发请求 —— 启停/删除的目标值与 API 调用都由
//! `list.rs` 与 `page.rs` 决定;本文件只把三个按钮的点击原样抛出去。

use dioxus::prelude::*;

use contract::api::admin::ChannelDto;

use super::shared::{
    BTN_DELETE_TITLE, BTN_DISABLE, BTN_EDIT, BTN_ENABLE, LBL_BASE_URL, LBL_REMARK,
    LBL_STATUS_DISABLED, LBL_STATUS_ENABLED, LBL_WEIGHT,
};
use crate::tab_page_groups::Badge;
/// 单个渠道卡片 (对齐 GroupCard / UserCard 规范)
///
/// 【是什么】渠道网格里的一张卡:头像首字母 + 名称/UUID + 状态徽标 + 指标行 + 底部三键。
///
/// 【做什么】把一个 `ChannelDto` 渲染成卡片,并在点击时把意图抛出;不决定启停的
/// 目标状态(由 `list.rs` 算好)、不持状态、不发网络。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互(全部为抛事件,组件内无网络):
/// - 点「编辑」(`data-testid="edit-channel"`)→ `on_edit.call(())`。
/// - 点「停用/启用」(`data-testid="toggle-channel"`,按钮文案随 `status == 1` 切换)
///   → `on_toggle.call(())`;目标状态由 `list.rs` 包装时决定。
/// - 点「删除」(`data-testid="delete-channel"`,悬停 `title` 为「删除渠道」)
///   → `on_delete.call(())`。无确认弹窗,直接向上抛。
///
/// 【样式】外壳 `rounded-xl border border-zinc-800 bg-zinc-900/60 p-4
/// transition-all duration-200`,悬停时 `hover:border-zinc-600 hover:bg-zinc-900/80`;
/// 首字母圆章 `h-9 w-9 rounded-full border-zinc-700 bg-zinc-800`;状态徽标启用为
/// 绿调 `border-emerald-500/30 bg-emerald-500/20 text-emerald-400`、停用为
/// `border-zinc-700 bg-zinc-800/80 text-zinc-400`;底部操作区 `mt-4 flex gap-1.5
/// border-t border-zinc-800 pt-3`,编辑/启停按钮各占 `flex-1`,删除为固定 `w-7` 的 ✕。
///
/// 【子组件组成】`crate::tab_page_groups::Badge`(状态 / 分组 / 密钥数三个胶囊)。
///
/// 【数据流】
/// - 对内(入):`channel`(`ChannelDto`,含 `name` / `key` / `channel_type` / `groups` /
///   `key_count` / `base_url` / `weight` / `remark` / `status`,由 `list.rs` 逐项传入);
///   `on_edit` / `on_toggle` / `on_delete` 三个 `EventHandler<()>`。
/// - 对外(出):三个回调分别抛回 `list.rs`,再由 `list.rs` 补上 key 转发给 `page.rs`
///   (编辑 → 开弹窗回填;启停 → `set_channel_status_api`;删除 → `delete_channel_api`)。
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
            LBL_STATUS_ENABLED,
            "border-emerald-500/30 bg-emerald-500/20 text-emerald-400",
        )
    } else {
        (
            LBL_STATUS_DISABLED,
            "border-zinc-700 bg-zinc-800/80 text-zinc-400",
        )
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
                        span { class: "shrink-0 text-zinc-400", "{LBL_BASE_URL}" }
                        span { class: "truncate font-mono text-zinc-300 max-w-[140px]", title: "{channel.base_url}", "{channel.base_url}" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "{LBL_WEIGHT}" }
                        span { class: "font-medium text-zinc-200", "{channel.weight}" }
                    }
                    if !channel.remark.is_empty() {
                        div { class: "flex justify-between gap-2",
                            span { class: "shrink-0 text-zinc-400", "{LBL_REMARK}" }
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
                    "{BTN_EDIT}"
                }
                button {
                    class: if is_enabled {
                        "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-amber-400 transition-colors hover:bg-zinc-700 hover:text-amber-300"
                    } else {
                        "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-emerald-400 transition-colors hover:bg-zinc-700 hover:text-emerald-300"
                    },
                    onclick: move |_| on_toggle.call(()),
                    "data-testid": "toggle-channel",
                    if is_enabled { "{BTN_DISABLE}" } else { "{BTN_ENABLE}" }
                }
                button {
                    class: "w-7 rounded-lg border border-zinc-800 bg-zinc-800/40 py-1.5 text-xs text-zinc-500 hover:text-red-400 hover:border-red-900/60 transition-colors flex items-center justify-center",
                    title: "{BTN_DELETE_TITLE}",
                    "data-testid": "delete-channel",
                    onclick: move |_| on_delete.call(()),
                    "✕"
                }
            }
        }
    }
}
