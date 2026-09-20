//! 兑换码卡片:单张兑换码的概览(状态徽标 + 面额条 + 停用操作)。
//!
//! 纯展示组件:数据以值传入(`RedRowFE`),复制预览/停用事件通过 `on_copy` /
//! `on_disable` 抛回页面;停用写回(DELETE /api/redemption/{key})在
//! `page` 的 `disable_red` 里。组件内部零 `use_signal`。
//!
//! 边界:不含网格排版与四态分支(在 `list.rs`)、不含「刚复制」高亮状态的持有
//! (在 `list.rs` 的 `copied_key`,本组件只接收 `is_just_copied` 布尔);
//! `Badge` 复用 `tab-page-groups` 的实现。

use dioxus::prelude::*;

use super::shared::{
    BTN_COPIED, BTN_COPY, BTN_DISABLE, BTN_DISABLED, BTN_REDEEMED, LBL_AVAILABLE_QUOTA,
    LBL_CARD_ARIA_PREFIX, LBL_CREATED, LBL_FACE_VALUE_PREFIX, LBL_REDEEMED_AT, LBL_REDEEMED_BY,
    LBL_STATUS_DISABLED, LBL_STATUS_UNKNOWN, LBL_STATUS_UNUSED, LBL_STATUS_USED, RedRowFE,
};
use crate::tab_page_groups::Badge;

/// 兑换码卡片。
///
/// 【是什么】一张兑换码概览卡:圆形 ¥ 图标 + 卡密预览 + key + 状态/面值徽标 +
/// 可用面额条 + 指标行(生成时间 / 兑换人 / 核销时间)+ 底部 [复制预览][停用] 操作区。
///
/// 【做什么】按 `item.status` 映射状态文案、配色与进度条宽度(1 未使用=emerald 满格 /
/// 2 已核销=zinc 归零 / 3 已停用=amber 40% / 异常值兜底为未知状态);核销人核销时间两行
/// 按数据有无条件渲染;底部第二按钮按状态在「停用」「已核销」「已停用」间三选一。
/// 不负责复制到剪贴板(只抛事件)、不负责停用请求、不负责判断高亮何时结束。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点「复制预览」/「已复制」按钮 → `on_copy(key)` 抛回列表,列表写入内部的
///   `copied_key` 令本卡 `<is_just_copied>` 变真并换文案与配色(本地,不发网络)。
/// - 点「停用」→ `on_disable(key)` 抛回页面,页面调 `disable_redemption_api` 成功后重拉。
/// - 已在「已核销」/「已停用」态时,第二个按钮为 `disabled` 占位,不可点。
/// 数据交互:本组件自身**不发任何网络请求**。
///
/// 【样式】外壳 class 取自共用样式壳 `ui::CARD_SHELL_CLASS`
/// (`group flex flex-col justify-between rounded-xl border border-zinc-800
/// bg-zinc-900/60 p-4`,悬停 `hover:border-zinc-600 hover:bg-zinc-900/80` 且
/// `transition-all duration-200`),带 `data-testid="redemption-card"`、
/// `role="listitem"`(父容器 `redemptions-list` 是 `role="list"`,故保留
/// listitem 而不用 CardShell 的 region);头像圈 `h-9 w-9 rounded-full border
/// border-zinc-700 bg-zinc-800`;卡密预览为
/// `truncate font-mono text-sm text-zinc-100`;面额条 `h-1.5 w-full rounded-full
/// bg-zinc-800` 内嵌 `transition-all duration-300` 的彩色进度;底部操作区
/// `mt-4 flex gap-1.5 border-t border-zinc-800 pt-3`;复制按钮常态 zinc 系、复制后
/// 切换为 emerald 高亮(`border-emerald-500/80 bg-emerald-950/60 text-emerald-300`)。
///
/// 【子组件组成】`Badge`(状态徽标 + 面值徽标,来自 `tab-page-groups`);其余为原生元素。
///
/// 【数据流】
/// - 对内(入):`item`(单条 `RedRowFE`:key / 卡密预览 / ¥面额 / status / 核销人 /
///   核销时间 / 生成时间,页面由 `map_redemption_view` 映射)、`is_just_copied`
///   (列表的 `copied_key` 是否等于本卡 key,决定复制按钮外观)、`on_copy` / `on_disable`。
/// - 对外(出):`on_copy(key)` → 列表 `copied_key.set(Some(key))`(纯本地视觉);
///   `on_disable(key)` → 页面 `disable_red`,调停用 API 后 `reload + 1` 或写 `err`。
#[component]
pub fn RedemptionCard(
    item: RedRowFE,
    is_just_copied: bool,
    on_copy: EventHandler<String>,
    on_disable: EventHandler<String>,
) -> Element {
    let preview = item.code_preview.clone();
    let _ = preview;

    // 状态语义对齐后端 admin-billing/redeem.rs 写库口径:
    // 1=未使用 / 2=已核销(redeem 写入) / 3=已停用(disable 写入)。
    let (status_text, status_tone, bar_tone, bar_pct) = match item.status {
        1 => (
            LBL_STATUS_UNUSED,
            "border-emerald-500/30 bg-emerald-500/20 text-emerald-400",
            "bg-emerald-500",
            100,
        ),
        2 => (
            LBL_STATUS_USED,
            "border-zinc-700 bg-zinc-800/80 text-zinc-400",
            "bg-zinc-700",
            0,
        ),
        3 => (
            LBL_STATUS_DISABLED,
            "border-amber-500/30 bg-amber-500/20 text-amber-400",
            "bg-amber-500",
            40,
        ),
        // 后端只写 1/2/3;异常值兜底按中性 zinc 展示,与 prototype 卡"未知状态"口径一致。
        _ => (
            LBL_STATUS_UNKNOWN,
            "border-zinc-700 bg-zinc-800/80 text-zinc-400",
            "bg-zinc-700",
            0,
        ),
    };

    let key_clone = item.key.clone();
    let disable_key = item.key.clone();

    rsx! {
        div {
            "data-testid": "redemption-card",
            role: "listitem",
            "aria-label": "{LBL_CARD_ARIA_PREFIX}{item.key}",
            // 外壳 class 与分组/用户卡共用 shell::CARD_SHELL_CLASS(逐字一致),
            // 不再在本文件复制一份字面量。
            class: "{ui::CARD_SHELL_CLASS}",
            div { class: "space-y-3",
                // 头部
                div { class: "flex items-start gap-3",
                    div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-zinc-700 bg-zinc-800 text-sm font-semibold text-zinc-200 group-hover:border-zinc-500 transition-colors",
                        "¥"
                    }
                    div { class: "min-w-0 flex-1",
                        div { class: "flex items-center justify-between gap-2",
                            h3 { class: "truncate font-mono text-sm font-medium text-zinc-100", "{item.code_preview}" }
                        }
                        p { class: "mt-0.5 truncate text-[11px] text-zinc-400 font-mono", "{item.key}" }
                    }
                }

                // 徽标行
                div { class: "flex flex-wrap gap-1.5",
                    Badge { text: status_text.to_string(), tone: status_tone }
                    Badge { text: format!("{LBL_FACE_VALUE_PREFIX}{:.2}", item.quota_cny), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-200 font-mono" }
                }

                // 额度有效条
                div { class: "space-y-1.5",
                    div { class: "flex justify-between gap-2 text-[11px]",
                        span { class: "text-zinc-400", "{LBL_AVAILABLE_QUOTA}" }
                        span { class: "whitespace-nowrap font-medium text-zinc-200 font-mono", "¥ {item.quota_cny:.2}" }
                    }
                    div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                        div { class: "h-full rounded-full {bar_tone} transition-all duration-300", style: "width: {bar_pct}%" }
                    }
                }

                // 详情指标行
                div { class: "space-y-1.5 text-xs pt-1",
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "{LBL_CREATED}" }
                        span { class: "font-mono text-zinc-400", "{item.created}" }
                    }
                    if let Some(by) = &item.redeemed_by {
                        div { class: "flex justify-between gap-2",
                            span { class: "shrink-0 text-zinc-400", "{LBL_REDEEMED_BY}" }
                            span { class: "font-medium text-zinc-200", "{by}" }
                        }
                    }
                    if !item.redeemed_at.is_empty() {
                        div { class: "flex justify-between gap-2",
                            span { class: "shrink-0 text-zinc-400", "{LBL_REDEEMED_AT}" }
                            span { class: "font-mono text-zinc-300", "{item.redeemed_at}" }
                        }
                    }
                }
            }

            // 底部操作区: [复制预览] [停用] — 后端仅支持停用(无硬删/无重新启用)
            div {
                class: "mt-4 flex gap-1.5 border-t border-zinc-800 pt-3",
                button {
                    "data-testid": "copy-redemption",
                    class: if is_just_copied {
                        "flex-1 rounded-lg border border-emerald-500/80 bg-emerald-950/60 py-1.5 text-xs text-emerald-300 transition-colors font-medium"
                    } else {
                        "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white"
                    },
                    onclick: move |_| on_copy.call(key_clone.clone()),
                    if is_just_copied { "{BTN_COPIED}" } else { "{BTN_COPY}" }
                }
                if item.status == 1 {
                    button {
                        "data-testid": "disable-redemption",
                        class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-amber-400 transition-colors hover:bg-zinc-700 hover:text-amber-300",
                        onclick: move |_| on_disable.call(disable_key.clone()),
                        "{BTN_DISABLE}"
                    }
                } else if item.status == 2 {
                    button {
                        "data-testid": "redeemed-redemption",
                        class: "flex-1 rounded-lg border border-zinc-800 bg-zinc-900 py-1.5 text-xs text-zinc-600 cursor-not-allowed",
                        disabled: true,
                        "{BTN_REDEEMED}"
                    }
                } else {
                    button {
                        "data-testid": "disabled-redemption",
                        class: "flex-1 rounded-lg border border-zinc-800 bg-zinc-900 py-1.5 text-xs text-zinc-600 cursor-not-allowed",
                        disabled: true,
                        "{BTN_DISABLED}"
                    }
                }
            }
        }
    }
}
