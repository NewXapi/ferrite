//! 分组筛选与批量操作区(编号段 2):标题 + 刷新/新建按钮 + 搜索框 +
//! 状态分级胶囊 + 批量点选 chips + 批量动作条。
//!
//! 纯交互组件:搜索词 / 分级档位 / 多选集合以 `Signal` prop 注入,组件内就地读写
//! (页面需要读同一份状态去算 `filtered`);刷新 / 新建 / 批量动作 / 清空以
//! `EventHandler` 抛回页面。组件内部零 `use_signal`。
//!
//! 边界:不做筛选计算(页面据此算 `filtered`)、不发网络请求(刷新与批量写回都在
//! `page.rs` 的 `make_bulk_toggle` / `reload` 闭包里)。

use dioxus::prelude::*;
use ui::SegmentedCapsule;

use contract::api::admin::GroupDto;

use super::shared::{
    BTN_BULK_CLEAR, BTN_BULK_DISABLE, BTN_BULK_ENABLE, BTN_NEW_GROUP, BTN_REFRESH, LBL_BULK_SELECT,
    MSG_BULK_SELECTED_PREFIX, MSG_BULK_SELECTED_SUFFIX, MSG_SEARCH_PLACEHOLDER, SEC_FILTER,
    SEC_FILTER_NOTE,
};

/// 筛选与操作区。
///
/// 【是什么】分组 tab 的编号段 2:一张带边框的筛选卡,内含标题行(刷新 / 新建
/// 按钮)、搜索输入框、状态分级胶囊、以及卡外的批量点选 chips 与批量动作条。
///
/// 【做什么】就地读写 `<search>` / `<filter_tier>` / `<selected>` 三个页面级
/// signal,渲染搜索框、分级胶囊、全量分组 chips;选中数 > 0 时追加批量动作条。
/// 不负责筛选计算、不负责拉数据、不负责批量写回(只把动作抛回页面)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 输入搜索词 → `search.set(输入值)`,页面据此重算 `filtered`(纯本地,不发网络)。
/// - 点分级胶囊 → `filter_tier.set(i)`,同上。
/// - 点分组 chip → 在 `selected` 里加入 / 移出该 key(本地增删,不发网络)。
/// - 点「刷新」/「新建分组」→ 分别调 `on_refresh` / `on_new` 抛回页面。
/// - 点「批量启用」/「批量停用」/「清除」→ 调 `on_bulk_enable` / `on_bulk_disable` /
///   `on_bulk_clear`;前两者触发页面顺序 await 多个 `set_group_status_api`。
/// 数据交互:本组件自身**不发任何网络请求**。
///
/// 【样式】外壳 `section#groups-sec-filter` 为 `scroll-mt-8 flex flex-col gap-4
/// rounded-xl border border-border bg-card p-5`(单张带边框深色卡);搜索框为
/// `w-full rounded-xl border border-border/80 bg-background`,聚焦时 `focus:border-border`;
/// 「新建分组」按钮为白底 `bg-primary text-primary-foreground` 主按钮;chip 选中态
/// `border-zinc-100 bg-primary text-primary-foreground font-semibold`,未选中态
/// `border-border bg-card`;批量动作条 `border-border/80 bg-background` 仅在
/// 有选中时渲染(启用按钮为 emerald 绿色系)。
///
/// 【子组件组成】`SegmentedCapsule`(状态分级胶囊);其余为原生 `section` / `input` /
/// `button`,无自定义子组件。
///
/// 【数据流】
/// - 对内(入):`groups`(全量分组,供 chips 点选,不经过关键词/分级筛选)、
///   `filter_options`(含各档计数的胶囊文案,页面派生)、`search` / `filter_tier` /
///   `selected`(页面持有的 Signal,双向就地读写)、`on_refresh` / `on_new` /
///   `on_bulk_enable` / `on_bulk_disable` / `on_bulk_clear`。
/// - 对外(出):`search` / `filter_tier` / `selected` 的写回改变页面派生结果与批量
///   目标集合;`on_refresh` → 页面 `reload + 1` 重拉;`on_new` → 页面清空表单字段并
///   置 `ModalState::New`;`on_bulk_*` → 页面读 `selected` 快照顺序调 API 并写入
///   `notice`;`on_bulk_clear` → 页面把 `selected` 置空。
#[component]
pub fn GroupsToolbar(
    groups: Vec<GroupDto>,
    filter_options: Vec<String>,
    search: Signal<String>,
    filter_tier: Signal<usize>,
    selected: Signal<Vec<String>>,
    on_refresh: EventHandler<()>,
    on_new: EventHandler<()>,
    on_bulk_enable: EventHandler<()>,
    on_bulk_disable: EventHandler<()>,
    on_bulk_clear: EventHandler<()>,
) -> Element {
    rsx! {
        section {
            id: "groups-sec-filter",
            class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-border bg-card p-5",
            div { class: "flex items-center justify-between gap-3",
                div { class: "flex items-center gap-2",
                    h2 { class: "{ui::TYPE_CARD_TITLE}", "{SEC_FILTER}" }
                    span { class: "{ui::TYPE_DESC}", "{SEC_FILTER_NOTE}" }
                }
                div { class: "flex items-center gap-2",
                    button {
                        class: "rounded-xl border border-border bg-background px-3.5 py-2 {ui::TYPE_DESC} transition-colors hover:border-border hover:text-foreground",
                        "data-testid": "refresh-groups",
                        onclick: move |_| on_refresh.call(()),
                        "{BTN_REFRESH}"
                    }
                    button {
                        class: "shrink-0 rounded-xl bg-primary px-4 py-2 {ui::TYPE_DESC} transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                        "data-testid": "new-group",
                        onclick: move |_| on_new.call(()),
                        "{BTN_NEW_GROUP}"
                    }
                }
            }

            input {
                class: "w-full rounded-xl border border-border/80 bg-background px-4 py-2.5 {ui::TYPE_BODY} placeholder:text-muted-foreground outline-none transition focus:border-border",
                r#type: "text",
                "data-testid": "group-search",
                placeholder: MSG_SEARCH_PLACEHOLDER,
                value: "{search}",
                oninput: move |e| search.set(e.value()),
            }

            // 分类胶囊
            div { class: "flex flex-wrap gap-2",
                SegmentedCapsule {
                    items: filter_options,
                    active: filter_tier(),
                    on_select: move |i: usize| filter_tier.set(i),
                }
            }

            // 批量多选区 (卡牌外): 列全部分组 chips, 点选加入/移出选中集合;
            // 选中数>0 时下方出现批量动作条 (批量启停 / 清除)。
            div { class: "space-y-2",
                p { class: "mb-1.5 {ui::TYPE_LABEL}", "{LBL_BULK_SELECT}" }
                div { class: "flex flex-wrap gap-1.5",
                    "data-testid": "bulk-select",
                    for g in groups {
                        {
                            let key = g.key.clone();
                            let gn = g.name.clone();
                            let picked = selected.peek().contains(&key);
                            let cls = if picked {
                                "border-zinc-100 bg-primary text-primary-foreground font-semibold"
                            } else {
                                "border-border bg-card text-foreground hover:border-border"
                            };
                            rsx! {
                                button {
                                    class: "rounded-lg border px-2.5 py-1 {ui::TYPE_DESC} transition-colors {cls}",
                                    "data-testid": "bulk-select-chip",
                                    onclick: move |_| {
                                        let mut s = selected.peek().to_vec();
                                        if let Some(pos) = s.iter().position(|k| *k == key) {
                                            s.remove(pos);
                                        } else {
                                            s.push(key.clone());
                                        }
                                        selected.set(s);
                                    },
                                    "{gn}"
                                }
                            }
                        }
                    }
                }
                // 批量动作条: 勾选后出现
                if !selected().is_empty() {
                    div { class: "flex flex-wrap items-center gap-2 rounded-xl border border-border/80 bg-background px-3 py-2.5",
                        "data-testid": "bulk-bar",
                        span { class: "{ui::TYPE_DESC}", "{MSG_BULK_SELECTED_PREFIX}{selected().len()}{MSG_BULK_SELECTED_SUFFIX}" }
                        button {
                            class: "rounded-lg border border-emerald-700/50 bg-success px-2.5 py-1 {ui::TYPE_DESC} {ui::C_SUCCESS} transition-colors hover:bg-success",
                            "data-testid": "bulk-enable",
                            onclick: move |_| on_bulk_enable.call(()),
                            "{BTN_BULK_ENABLE}"
                        }
                        button {
                            class: "rounded-lg border border-border/80 bg-secondary/60 px-2.5 py-1 {ui::TYPE_DESC} transition-colors hover:bg-secondary",
                            "data-testid": "bulk-disable",
                            onclick: move |_| on_bulk_disable.call(()),
                            "{BTN_BULK_DISABLE}"
                        }
                        button {
                            class: "rounded-lg border border-border/80 px-2.5 py-1 {ui::TYPE_DESC} transition-colors hover:bg-secondary hover:text-foreground",
                            "data-testid": "bulk-clear",
                            onclick: move |_| on_bulk_clear.call(()),
                            "{BTN_BULK_CLEAR}"
                        }
                    }
                }
            }
        }
    }
}
