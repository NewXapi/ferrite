//! 兑换码卡片网格区(编号段 3):标题计数 + 四态分支 + RedemptionCard 网格。
//!
//! 纯展示组件:数据以值传入;交互通过 EventHandler 抛回页面。
//! copied_key(复制高亮)是纯列表内部视觉状态,组件内部持有,不经过页面。
//!
//! 边界:筛选计算、拉取与停用写回都在 `page.rs`;卡片本体与状态配色在 `card.rs`;
//! 本文件只做四态分支与网格排版。

use dioxus::prelude::*;

use super::card::RedemptionCard;
use super::shared::{
    BTN_RETRY, LBL_ERROR_ARIA, LBL_LIST_ARIA, MSG_EMPTY, MSG_LOAD_FAILED, MSG_LOADING_LIST,
    OPT_BADGE_LOADING, RedRowFE, SEC_LIST,
};

/// 兑换码卡片网格区:错误 / 加载 / 空 / 网格 四态。
///
/// 【是什么】兑换码 tab 的编号段 3:标题行 + 计数徽标 + 四态分支,有数据时把每条
/// `RedRowFE` 铺成 `RedemptionCard`。
///
/// 【做什么】按 `err` / `loading` / `filtered_rows` 渲染四态。不负责筛选、不负责拉数据、
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
/// 标题行左侧 `text-lg font-medium text-foreground`,右侧计数胶囊 `rounded-full
/// bg-secondary`(加载时 `OPT_BADGE_LOADING`,否则 `N 张`);错误态红底
/// `rounded-2xl border border-destructive bg-destructive px-4 py-6` 且 `role="alert"`;
/// 加载/空态为虚线描边 `rounded-2xl border border-dashed border-border
/// bg-card/50 py-16`;网格为 `grid grid-cols-1 gap-3 md:grid-cols-3
/// lg:grid-cols-5`(手机 1 / 中屏 3 / 大屏 5 列)。
///
/// 【子组件组成】`card::RedemptionCard`(可操作兑换码卡,外壳用共用样式壳);
/// 四态块为原生 `div` / `p` / `button`。
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

    // 分页：列表内部 UI 状态（不跨组件）；筛选后条数变小时 clamp 到最后一页，
    // 不落空页（详见 aliases list 同款注释）。
    let mut page = use_signal(|| 0usize);
    let visible = ui::page_slice(&filtered_rows, page(), ui::CARD_PAGE_SIZE).to_vec();

    rsx! {
        section {
            id: "reds-sec-list",
            "data-testid": "redemptions-list",
            role: "list",
            "aria-label": LBL_LIST_ARIA,
            class: "scroll-mt-8 space-y-4",
            ui::SectionHeader {
                title: SEC_LIST.to_string(),
                badge: if loading { OPT_BADGE_LOADING.to_string() } else { format!("{} 张", filtered_rows.len()) },
                trailing: rsx! {
                    ui::Pager {
                        total: filtered_rows.len(),
                        page,
                        on_change: move |p| page.set(p),
                        testid: "redemptions-pager",
                    }
                },
            }

            if let Some(e) = err {
                div {
                    "data-testid": "redemptions-error",
                    role: "alert",
                    "aria-label": LBL_ERROR_ARIA,
                    class: "rounded-2xl border border-destructive bg-destructive px-4 py-6 text-center",
                    p { class: "text-sm {ui::C_DANGER}", "{MSG_LOAD_FAILED}" }
                    p { class: "mt-1 text-xs {ui::C_DANGER}", "{e}" }
                    button {
                        "data-testid": "retry-redemptions",
                        class: "mt-3 rounded-xl border border-border px-3 py-1.5 {ui::TYPE_DESC} hover:bg-secondary",
                        onclick: on_retry,
                        "{BTN_RETRY}"
                    }
                }
            } else if loading {
                div { class: "rounded-2xl border border-dashed border-border bg-card/50 py-16 text-center",
                    p { class: "{ui::C_MUTED}", "{MSG_LOADING_LIST}" }
                }
            } else if filtered_rows.is_empty() {
                div {
                    "data-testid": "redemptions-empty",
                    class: "rounded-2xl border border-dashed border-border bg-card/50 py-16 text-center",
                    p { class: "{ui::C_MUTED}", "{MSG_EMPTY}" }
                }
            } else {
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                    for r in visible {
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
