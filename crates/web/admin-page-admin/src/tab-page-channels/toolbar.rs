//! 渠道筛选与操作区(编号段 2):标题 + 刷新/新建按钮 + 搜索框 + 状态胶囊。
//!
//! search / filter_tier / reload 由页面持有(页面据此算 filtered 并重拉),
//! 组件通过 Signal prop 就地读写;on_new(开弹窗)是跨组件交互,由页面闭包注入。
//! 组件内部零 use_signal。
//!
//! 边界:本区只收集筛选条件与发出刷新/新建意图;真正的过滤由 `page.rs` 调用
//! `shared::filter_channels` 完成,重拉由页面的 `use_effect` 监听 `reload` 后执行。

use dioxus::prelude::*;
use ui::SegmentedCapsule;

use super::shared::{
    BTN_NEW_CHANNEL, BTN_REFRESH, MSG_SEARCH_PLACEHOLDER, SEC_FILTER, SEC_FILTER_NOTE,
};

/// 渠道筛选与操作区。
///
/// 【是什么】渠道 tab 的编号段 2:区段标题 + 刷新/新建按钮 + 搜索框 + 按状态分档的胶囊。
///
/// 【做什么】承载搜索词、状态档位、刷新、新建四个入口;不负责过滤列表(页面调
/// `filter_channels` 算),不负责真正的拉取(由页面 effect 响应 `reload` 执行)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 在搜索框输入 → `oninput` 写 `search`(页面算 filtered),不发网络。
/// - 点状态胶囊某档 → `SegmentedCapsule::on_select` 写 `filter_tier`,不发网络。
/// - 点「刷新」(`data-testid="refresh-channels"`)→ `reload` 自增 1,页面 `use_effect`
///   监听到变化后重拉(GET),已有数据时走 `refreshing` 不清列表。
/// - 点「新建渠道」(`data-testid="new-channel"`)→ 调 `on_new`,页面 `open_new` 重置表单并开弹窗。
///
/// 【样式】外壳 `section#channels-sec-filter`,class 为
/// `scroll-mt-8 flex flex-col gap-4 rounded-xl border border-border bg-card p-5`
/// (ScrollSpy 锚点 + 圆角描边面板)。刷新按钮描边 `border-border bg-background`
/// hover 转 `border-border`,新建按钮白底 `bg-primary text-primary-foreground` hover `bg-zinc-200`;
/// 搜索框 `rounded-xl border-border/80 bg-background`,聚焦 `focus:border-border`;
/// 胶囊行 `flex flex-wrap gap-2` 允许换行。
///
/// 【子组件组成】`ui::SegmentedCapsule`(状态分档胶囊);按钮与输入框为原生元素。
///
/// 【数据流】
/// - 对内(入):`search`(页面持有的搜索词)、`filter_tier`(0 全部 / 1 启用中 /
///   2 已停用)、`reload`(重拉计数)、`filter_options`(页面派生的含计数胶囊文案)、
///   `on_new`(页面注入的开弹窗回调)。
/// - 对外(出):`search` / `filter_tier` / `reload` 就地写回页面状态;`on_new` 抛回
///   页面,由 `open_new` 重置 `f_*` 表单与候选池后置 `modal_state = New`。
#[component]
pub fn ChannelsToolbarSection(
    /// 搜索词(页面据此算 filtered,Signal 双向绑定)
    search: Signal<String>,
    /// 状态分级档位(0 全部 / 1 启用 / 2 停用)
    filter_tier: Signal<usize>,
    /// 重拉计数:刷新按钮 +1 触发页面 use_effect 重拉
    reload: Signal<u32>,
    /// 状态胶囊文案(含各档计数,页面派生)
    filter_options: Vec<String>,
    /// 「新建渠道」:开弹窗,跨组件交互,由页面闭包处理
    on_new: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        section {
            id: "channels-sec-filter",
            class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-border bg-card p-5",
            div { class: "flex items-center justify-between gap-3",
                div { class: "flex items-center gap-2",
                    h2 { class: "{ui::TYPE_CARD_TITLE}", "{SEC_FILTER}" }
                    span { class: "{ui::TYPE_DESC}", "{SEC_FILTER_NOTE}" }
                }
                div { class: "flex items-center gap-2",
                    button {
                        class: "rounded-xl border border-border bg-background px-3.5 py-2 {ui::TYPE_DESC} transition-colors hover:border-border hover:text-foreground",
                        "data-testid": "refresh-channels",
                        onclick: move |_| reload.set(reload() + 1),
                        "{BTN_REFRESH}"
                    }
                    button {
                        class: "shrink-0 rounded-xl bg-primary px-4 py-2 {ui::TYPE_DESC} transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                        "data-testid": "new-channel",
                        onclick: on_new,
                        "{BTN_NEW_CHANNEL}"
                    }
                }
            }

            input {
                class: "w-full rounded-xl border border-border/80 bg-background px-4 py-2.5 {ui::TYPE_BODY} placeholder:text-muted-foreground outline-none transition focus:border-border",
                r#type: "text",
                placeholder: "{MSG_SEARCH_PLACEHOLDER}",
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
