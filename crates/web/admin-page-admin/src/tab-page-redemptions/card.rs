//! 兑换码卡片:单张兑换码的概览(状态徽标 + 面额条 + 停用操作)。
//!
//! 纯展示组件:数据以值传入(`RedRowFE`),复制预览/停用事件通过 `on_copy` /
//! `on_disable` 抛回页面;停用写回(DELETE /api/redemption/{key})在
//! `page` 的 `disable_red` 里。

use dioxus::prelude::*;

use super::shared::RedRowFE;
use crate::tab_page_groups::Badge;

/// 兑换码卡片
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
            "未使用",
            "border-emerald-500/30 bg-emerald-500/20 text-emerald-400",
            "bg-emerald-500",
            100,
        ),
        2 => (
            "已核销",
            "border-zinc-700 bg-zinc-800/80 text-zinc-400",
            "bg-zinc-700",
            0,
        ),
        3 => (
            "已停用",
            "border-amber-500/30 bg-amber-500/20 text-amber-400",
            "bg-amber-500",
            40,
        ),
        // 后端只写 1/2/3;异常值兜底按中性 zinc 展示,与 prototype 卡"未知状态"口径一致。
        _ => (
            "未知状态",
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
            "aria-label": "兑换码 {item.key}",
            class: "group flex flex-col justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
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
                    Badge { text: format!("面值 ¥{:.2}", item.quota_cny), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-200 font-mono" }
                }

                // 额度有效条
                div { class: "space-y-1.5",
                    div { class: "flex justify-between gap-2 text-[11px]",
                        span { class: "text-zinc-400", "可用面额" }
                        span { class: "whitespace-nowrap font-medium text-zinc-200 font-mono", "¥ {item.quota_cny:.2}" }
                    }
                    div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                        div { class: "h-full rounded-full {bar_tone} transition-all duration-300", style: "width: {bar_pct}%" }
                    }
                }

                // 详情指标行
                div { class: "space-y-1.5 text-xs pt-1",
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "生成时间" }
                        span { class: "font-mono text-zinc-400", "{item.created}" }
                    }
                    if let Some(by) = &item.redeemed_by {
                        div { class: "flex justify-between gap-2",
                            span { class: "shrink-0 text-zinc-400", "兑换人" }
                            span { class: "font-medium text-zinc-200", "{by}" }
                        }
                    }
                    if !item.redeemed_at.is_empty() {
                        div { class: "flex justify-between gap-2",
                            span { class: "shrink-0 text-zinc-400", "核销时间" }
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
                    if is_just_copied { "已复制" } else { "复制预览" }
                }
                if item.status == 1 {
                    button {
                        "data-testid": "disable-redemption",
                        class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-amber-400 transition-colors hover:bg-zinc-700 hover:text-amber-300",
                        onclick: move |_| on_disable.call(disable_key.clone()),
                        "停用"
                    }
                } else if item.status == 2 {
                    button {
                        "data-testid": "redeemed-redemption",
                        class: "flex-1 rounded-lg border border-zinc-800 bg-zinc-900 py-1.5 text-xs text-zinc-600 cursor-not-allowed",
                        disabled: true,
                        "已核销"
                    }
                } else {
                    button {
                        "data-testid": "disabled-redemption",
                        class: "flex-1 rounded-lg border border-zinc-800 bg-zinc-900 py-1.5 text-xs text-zinc-600 cursor-not-allowed",
                        disabled: true,
                        "已停用"
                    }
                }
            }
        }
    }
}
