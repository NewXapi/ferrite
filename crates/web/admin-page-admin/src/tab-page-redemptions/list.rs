//! 兑换码卡片网格区(编号段 3):标题计数 + 四态分支 + 新卡示例 + RedemptionCard 网格。
//!
//! 纯展示组件:数据以值传入;交互通过 EventHandler 抛回页面。
//! copied_key(复制高亮)是纯列表内部视觉状态,组件内部持有,不经过页面。
//!
//! 边界:筛选计算、拉取与停用写回都在 `page.rs`;卡片本体与状态配色在 `card.rs`;
//! 本文件只做四态分支、示例区与网格排版。

use dioxus::prelude::*;
use ui::RedemptionCard as PrototypeRedemptionCard;

use super::card::RedemptionCard;
use super::shared::{
    BTN_RETRY, LBL_ERROR_ARIA, LBL_LIST_ARIA, LBL_PROTOTYPE_REGION, MSG_EMPTY, MSG_LOAD_FAILED,
    MSG_LOADING_LIST, OPT_BADGE_LOADING, RedRowFE, SEC_LIST,
};

/// 兑换码卡片网格区:错误 / 加载 / 空 / 网格 四态。
///
/// 【是什么】兑换码 tab 的编号段 3:标题行 + 计数徽标 + 四态分支,有数据时先渲染一张
/// 只读原型卡示例,再把每条 `RedRowFE` 铺成 `RedemptionCard`。
///
/// 【做什么】按 `err` / `loading` / `filtered_rows` 渲染四态;并在有数据时把首行同时
/// 映射成 `ui::RedemptionCard` 原型(只读、展示设计定稿样式)。不负责筛选、不负责拉数据、
/// 不负责停用请求(只抛 key)、不负责复制高亮的跨组件传播。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点错误态「重试」→ `on_retry(MouseEvent)` 抛回页面,页面 `reload + 1` 触发重拉。
/// - 点卡片「复制预览」→ 列表内部 `copied_key.set(Some(key))`,该卡高亮并换文案
///   `已复制`(纯本地视觉状态,组件自己持有,不下沉到页面)。
/// - 点卡片「停用」→ `on_disable(key)` 抛回页面,页面调停用 API 后重拉或写 `err`。
/// 数据交互:本组件自身**不发任何网络请求**。
///
/// 【样式】外壳 `section#reds-sec-list` 为 `scroll-mt-8 space-y-4`,并带
/// `data-testid="redemptions-list"`、`role="list"`、`aria-label=LBL_LIST_ARIA`;
/// 标题行左侧 `text-lg font-medium text-zinc-100`,右侧计数胶囊 `rounded-full
/// bg-zinc-800`(加载时 `OPT_BADGE_LOADING`,否则 `N 张`);错误态红底
/// `rounded-2xl border border-red-800/60 bg-red-950/40 px-4 py-6` 且 `role="alert"`;
/// 加载/空态为虚线描边 `rounded-2xl border border-dashed border-zinc-700
/// bg-zinc-900/50 py-16`;示例区与网格均为 `grid grid-cols-1 gap-3 md:grid-cols-3
/// lg:grid-cols-5`(手机 1 / 中屏 3 / 大屏 5 列)。
///
/// 【子组件组成】`ui::RedemptionCard`(别名 `PrototypeRedemptionCard`,只读原型示例)、
/// `card::RedemptionCard`(可操作兑换码卡);四态块为原生 `div` / `p` / `button`。
///
/// 【数据流】
/// - 对内(入):`loading` / `err`(页面 effect 的加载与失败态;`err` 优先于 `loading` 渲染)、
///   `filtered_rows`(页面按搜索词 + 状态档位筛好的 `RedRowFE`,只用于计数与渲染)、
///   `on_disable` / `on_retry`(页面闭包)。
/// - 对外(出):`on_disable(key)` → 页面 `disable_red`(停用 API + 重拉);
///   `on_retry` → 页面 `reload + 1`;`copied_key` 只在组件内部读写,不外露。
///
/// 状态块:`copied_key` 是「刚复制」高亮的内部视觉状态,不参与任何跨组件交互,
/// 故留在组件内 `use_signal`;点击复制后写入,卡片据此切换按钮文案与配色。
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
            "aria-label": LBL_LIST_ARIA,
            class: "scroll-mt-8 space-y-4",
            div { class: "flex items-center justify-between",
                h2 { class: "text-lg font-medium text-zinc-100", "{SEC_LIST}" }
                if loading {
                    span { class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400", "{OPT_BADGE_LOADING}" }
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
                    "aria-label": LBL_ERROR_ARIA,
                    class: "rounded-2xl border border-red-800/60 bg-red-950/40 px-4 py-6 text-center",
                    p { class: "text-sm text-red-300", "{MSG_LOAD_FAILED}" }
                    p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                    button {
                        "data-testid": "retry-redemptions",
                        class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                        onclick: on_retry,
                        "{BTN_RETRY}"
                    }
                }
            } else if loading {
                div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "{MSG_LOADING_LIST}" }
                }
            } else if filtered_rows.is_empty() {
                div {
                    "data-testid": "redemptions-empty",
                    class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "{MSG_EMPTY}" }
                }
            } else {
                if let Some(row) = filtered_rows.first().cloned() {
                    {
                        let redeemed_at = (!row.redeemed_at.is_empty()).then_some(row.redeemed_at.clone());
                        rsx! {
                            div {
                                class: "mb-4 grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                                role: "region",
                                "aria-label": LBL_PROTOTYPE_REGION,
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
