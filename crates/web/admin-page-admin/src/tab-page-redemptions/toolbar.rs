//! 兑换码筛选与操作区(编号段 2):标题 + 生成按钮 + 搜索框 + 状态胶囊。
//!
//! search / filter_tier / reload 由页面持有,组件通过 Signal prop 就地读写;
//! on_generate(开生成弹窗并预填表单)是跨组件交互,由页面闭包注入。
//! 组件内部零 use_signal。
//!
//! 边界:筛选计算与生成请求都在 `page.rs`;本文件不含列表四态与卡片(在 `list.rs` /
//! `card.rs`),不含弹窗(在 `modal.rs`)。

use dioxus::prelude::*;
use ui::SegmentedCapsule;

use super::shared::{
    BTN_GENERATE, LBL_FILTER_ARIA, MSG_SEARCH_PLACEHOLDER, SEC_FILTER, SEC_FILTER_NOTE,
};

/// 兑换码筛选与操作区。
///
/// 【是什么】兑换码 tab 的编号段 2:一张带边框的筛选卡,内含标题行(标题 + 说明 +
/// 右侧「生成兑换码」按钮)、搜索输入框、状态分级胶囊。
///
/// 【做什么】就地读写页面的 `search` / `filter_tier` signal,渲染搜索框与四档状态胶囊
/// (全部 / 未使用 / 已核销 / 已停用)。不负责筛选计算(页面据此算 `filtered_rows`)、
/// 不负责拉数据、不负责任何写回请求。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 输入搜索词 → `search.set(输入值)`,页面据此重算 `filtered_rows`(纯本地,不发网络)。
/// - 点状态胶囊 → `filter_tier.set(i)`,同上。
/// - 点「生成兑换码」→ `on_generate` 抛回页面,页面预填默认面额/数量并打开生成弹窗。
/// 数据交互:本组件自身**不发任何网络请求**。
///
/// 【样式】外壳 `section#reds-sec-filter` 为 `scroll-mt-8 flex flex-col gap-4
/// rounded-xl border border-zinc-800 bg-zinc-900 p-5`,带 `data-testid="redemptions-filter"`、
/// `role="search"`、`aria-label=LBL_FILTER_ARIA`;标题 `text-sm font-medium text-zinc-300`
/// 旁挂 `text-xs text-zinc-500` 说明;「生成兑换码」为白底 `bg-white text-zinc-900` 主按钮;
/// 搜索框 `w-full rounded-xl border border-zinc-700/80 bg-zinc-950`,聚焦 `focus:border-zinc-500`。
///
/// 【子组件组成】`SegmentedCapsule`(状态分级胶囊);其余为原生 `section` / `input` / `button`。
///
/// 【数据流】
/// - 对内(入):`search`(搜索词)、`filter_tier`(0 全部 / 1 未使用 / 2 已核销 / 3 已停用)、
///   `reload`(重拉计数,本区暂无刷新按钮,仅保持与 siblings 同形签名,未参与渲染)、
///   `filter_options`(含各档计数的胶囊文案,页面派生)、`on_generate`。
/// - 对外(出):`search` / `filter_tier` 写回改变页面派生结果;`on_generate` → 页面
///   `open_generate`(清空并预填 `f_count`/`f_quota` 后置 `RedModalState::Generate`)。
#[component]
pub fn RedemptionsToolbarSection(
    /// 搜索词(页面据此算 filtered,Signal 双向绑定)
    search: Signal<String>,
    /// 状态分级档位(0 全部 / 1 未使用 / 2 已核销 / 3 已停用)
    filter_tier: Signal<usize>,
    /// 重拉计数:预留(本区暂无刷新按钮,与 siblings 保持同形签名)
    reload: Signal<u32>,
    /// 状态胶囊文案(含各档计数,页面派生)
    filter_options: Vec<String>,
    /// 「生成兑换码」:开生成弹窗,跨组件交互,由页面闭包处理
    on_generate: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        section {
            id: "reds-sec-filter",
            "data-testid": "redemptions-filter",
            role: "search",
            "aria-label": LBL_FILTER_ARIA,
            class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
            div { class: "flex items-center justify-between gap-3",
                div { class: "flex items-center gap-2",
                    h2 { class: "{ui::TYPE_CARD_TITLE}", "{SEC_FILTER}" }
                    span { class: "{ui::TYPE_DESC}", "{SEC_FILTER_NOTE}" }
                }
                button {
                    "data-testid": "generate-redemptions",
                    class: "shrink-0 rounded-xl bg-white px-4 py-2 {ui::TYPE_DESC} transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                    onclick: on_generate,
                    "{BTN_GENERATE}"
                }
            }

            input {
                "data-testid": "redemptions-search",
                class: "w-full rounded-xl border border-zinc-700/80 bg-zinc-950 px-4 py-2.5 {ui::TYPE_BODY} placeholder:text-zinc-500 outline-none transition focus:border-zinc-500",
                r#type: "text",
                placeholder: MSG_SEARCH_PLACEHOLDER,
                value: "{search}",
                oninput: move |e| search.set(e.value()),
            }
            div { class: "flex flex-wrap gap-2",
                SegmentedCapsule {
                    items: filter_options,
                    active: filter_tier(),
                    on_select: move |i: usize| filter_tier.set(i),
                }
            }
        }
    }
}
