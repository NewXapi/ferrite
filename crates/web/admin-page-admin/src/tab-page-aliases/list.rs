//! 别名卡片网格区(编号段 3):标题计数 + 四态分支 + 新卡示例 + AliasCard 网格。
//!
//! 纯展示组件:数据(load/err/filtered/分组/通道开关)以值传入;
//! 交互通过 EventHandler 抛回页面。on_mode_change 收 (alias_key, mode) 二元组,
//! 对每张卡包装成单参闭包 —— 取代原先页面里的 make_mode_handler 闭包工厂。

use contract::api::admin::GroupDto;
use dioxus::prelude::*;
use ui::AliasCard as PrototypeAliasCard;

use super::card::AliasCard;
use super::shared::{AliasItem, PriceMode, SEC_LIST, usable_groups_for};

/// 别名卡片网格区:错误 / 加载 / 空 / 网格 四态。
#[component]
pub fn AliasesListSection(
    /// 列表加载中(骨架态)
    loading: bool,
    /// 拉取失败摘要(错误态;Some 时优先于 loading 渲染)
    err: Option<String>,
    /// 筛选后的 (原始下标, 条目) 列表,页面派生
    filtered: Vec<(usize, AliasItem)>,
    /// 全部分组(卡片展示「哪些分组可用此别名 + 各分组倍率」)
    groups: Vec<GroupDto>,
    /// 各补充通道启用状态(与弹窗「基本」tab 胶囊开关共享,页面持有)
    c_output_on: bool,
    c_cache_read_on: bool,
    c_cache_write_on: bool,
    c_completion_on: bool,
    /// per-card 定价模式切换:(alias_key, 新模式),页面写回 rows
    on_mode_change: EventHandler<(String, PriceMode)>,
    /// 请求编辑(开弹窗回填)
    on_edit: EventHandler<String>,
    /// 请求删除
    on_delete: EventHandler<String>,
    /// 错误态「重试」
    on_retry: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        section { id: "aliases-sec-list", class: "scroll-mt-8 space-y-4",
            div { class: "flex items-center justify-between",
                h2 { class: "text-lg font-medium text-zinc-100", "{SEC_LIST}" }
                span { class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400",
                    if loading { "加载中…" } else { "{filtered.len()} 个" }
                }
            }

            if let Some(e) = err {
                div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 py-10 text-center",
                    p { class: "text-sm text-red-300", "加载别名失败" }
                    p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                    button {
                        class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                        onclick: on_retry,
                        "重试"
                    }
                }
            } else if loading {
                div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "正在加载模型别名…" }
                }
            } else if filtered.is_empty() {
                div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "没有匹配的模型别名" }
                }
            } else {
                if let Some((prototype_index, prototype_item)) = filtered.first() {
                    {
                        // 新卡示例仅消费当前筛选结果的首条真实数据;旧卡片网格与其写路径保持不变。
                        let prototype_key = prototype_item.key.clone();
                        let prototype_alias = prototype_item.row.alias.clone();
                        let prototype_display = prototype_item.row.display.clone();
                        let prototype_input_per_1k = prototype_item.row.input_per_1k;
                        let prototype_output_per_1k = prototype_item.row.output_per_1k;
                        let prototype_multiplier = prototype_item.row.multiplier;
                        let prototype_index = *prototype_index;
                        // 分组可用性:白名单为空(全可用)或显式包含该别名,
                        // 与旧卡片网格共用 usable_groups_for 判定。
                        let prototype_usable_groups =
                            usable_groups_for(&prototype_alias, &groups);
                        rsx! {
                            div {
                                class: "mb-4 grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                                role: "region",
                                "aria-label": "别名新卡示例",
                                "data-testid": "alias-card-prototype",
                                PrototypeAliasCard {
                                    alias: prototype_alias,
                                    display: prototype_display,
                                    input_per_1k: prototype_input_per_1k,
                                    output_per_1k: prototype_output_per_1k,
                                    multiplier: prototype_multiplier,
                                    index: prototype_index,
                                    usable_groups: prototype_usable_groups,
                                    alias_key: prototype_key.clone(),
                                }
                            }
                        }
                    }
                }
                div {
                    class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                    role: "list",
                    "aria-label": "别名列表",
                    "data-testid": "aliases-list",
                    for (idx, it) in filtered {
                        {
                            let key_ref = it.key.clone();
                            rsx! {
                                AliasCard {
                                    key: "{it.key}",
                                    alias_key: it.key,
                                    alias: it.row,
                                    index: idx,
                                    price_mode: it.price_mode,
                                    groups: groups.clone(),
                                    c_output_on,
                                    c_cache_read_on,
                                    c_cache_write_on,
                                    c_completion_on,
                                    on_mode_change: move |mode| on_mode_change.call((key_ref.clone(), mode)),
                                    on_edit,
                                    on_delete,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
