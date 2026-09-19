//! 兑换码卡片网格区(编号段 3):标题计数 + 四态分支 + 新卡示例 + RedemptionCard 网格。
//!
//! 纯展示组件:数据以值传入;交互通过 EventHandler 抛回页面。
//! copied_key(复制高亮)是纯列表内部视觉状态,组件内部持有,不经过页面。

use dioxus::prelude::*;
use ui::RedemptionCard as PrototypeRedemptionCard;

use super::card::RedemptionCard;
use super::shared::{RedRowFE, SEC_LIST};

/// 兑换码卡片网格区:错误 / 加载 / 空 / 网格 四态。
#[component]
pub fn RedemptionsListSection(
    /// 列表加载中(骨架态)
    loading: bool,
    /// 拉取失败摘要(错误态;Some 时优先于 loading 渲染)
    err: Option<String>,
    /// 筛选后的兑换码列表,页面派生
    filtered_rows: Vec<RedRowFE>,
    /// 请求停用(DELETE 语义,后端 status→3)
    on_disable: EventHandler<String>,
    /// 错误态「重试」
    on_retry: EventHandler<MouseEvent>,
) -> Element {
    // 「刚复制」高亮是本网格的内部视觉状态(点击复制 → 该卡短暂高亮),
    // 不参与任何跨组件交互,组件内部持有。
    let mut copied_key = use_signal(|| None::<String>);

    rsx! {
        section {
            id: "reds-sec-list",
            "data-testid": "redemptions-list",
            role: "list",
            "aria-label": "兑换码列表",
            class: "scroll-mt-8 space-y-4",
            div { class: "flex items-center justify-between",
                h2 { class: "text-lg font-medium text-zinc-100", "{SEC_LIST}" }
                if loading {
                    span { class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400", "加载中…" }
                } else {
                    span { class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400",
                        "{filtered_rows.len()} 张"
                    }
                }
            }

            if let Some(e) = err {
                div {
                    "data-testid": "redemptions-error",
                    role: "alert",
                    "aria-label": "兑换码加载失败",
                    class: "rounded-2xl border border-red-800/60 bg-red-950/40 px-4 py-6 text-center",
                    p { class: "text-sm text-red-300", "加载兑换码失败" }
                    p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                    button {
                        "data-testid": "retry-redemptions",
                        class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                        onclick: on_retry,
                        "重试"
                    }
                }
            } else if loading {
                div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "正在加载兑换码…" }
                }
            } else if filtered_rows.is_empty() {
                div {
                    "data-testid": "redemptions-empty",
                    class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "没有匹配的兑换码" }
                }
            } else {
                if let Some(row) = filtered_rows.first().cloned() {
                    {
                        let redeemed_at = (!row.redeemed_at.is_empty()).then_some(row.redeemed_at.clone());
                        rsx! {
                            div {
                                class: "mb-4 grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                                role: "region",
                                "aria-label": "新卡示例",
                                "data-testid": "redemption-card-prototype",
                                PrototypeRedemptionCard {
                                    redemption_key: row.key,
                                    code_preview: row.code_preview,
                                    quota_cny: row.quota_cny,
                                    status: i16::from(row.status),
                                    redeemed_by: row.redeemed_by,
                                    redeemed_at,
                                    created_at: row.created,
                                }
                            }
                        }
                    }
                }
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                    for r in filtered_rows {
                        {
                            let is_just_copied = copied_key() == Some(r.key.clone());
                            rsx! {
                                RedemptionCard {
                                    key: "{r.key}",
                                    item: r,
                                    is_just_copied: is_just_copied,
                                    on_copy: move |k: String| {
                                        copied_key.set(Some(k));
                                    },
                                    on_disable,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
