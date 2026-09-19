//! 别名卡片网格区(编号段 3):标题计数 + 四态分支 + AliasCard 网格。
//!
//! 纯展示组件:数据(load/err/filtered/分组/通道开关)以值传入;
//! 交互通过 EventHandler 抛回页面。on_mode_change 收 (alias_key, mode) 二元组,
//! 对每张卡包装成单参闭包 —— 取代原先页面里的 make_mode_handler 闭包工厂。
//!
//! 卡片本体是 ui-components 的 `AliasCard`（四页签 + 共享外壳）；
//! 本文件只做四态分支与网格排版，不再自带卡片样式。

use contract::api::admin::GroupDto;
use dioxus::prelude::*;
use ui::{CardGrid, DangerBlock, GhostButton, PlaceholderBlock, SectionHeader};

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
    // 通道开关与编辑/删除回调透传给卡片；卡片当前不渲染编辑入口，
    // 保留绑定以维持与页面写路径的接线（后续 Popover 编辑接入点）。
    let _ = (
        c_output_on,
        c_cache_read_on,
        c_cache_write_on,
        c_completion_on,
        on_edit,
        on_delete,
    );

    let badge = if loading {
        "加载中…".to_string()
    } else {
        format!("{} 个", filtered.len())
    };

    rsx! {
        section { id: "aliases-sec-list", class: "scroll-mt-8 space-y-4",
            SectionHeader { title: SEC_LIST, badge: badge }

            if let Some(e) = err {
                DangerBlock {
                    title: "加载别名失败",
                    detail: e,
                    GhostButton { label: "重试", onclick: on_retry }
                }
            } else if loading {
                PlaceholderBlock { message: "正在加载模型别名…" }
            } else if filtered.is_empty() {
                PlaceholderBlock { message: "没有匹配的模型别名" }
            } else {
                CardGrid { aria_label: "别名列表", testid: "aliases-list",
                    for (idx, it) in filtered {
                        {
                            let key_ref = it.key.clone();
                            let usable = usable_groups_for(&it.row.alias, &groups);
                            let display = it.row.display.clone();
                            let mode = it.price_mode;
                            rsx! {
                                ui::AliasCard {
                                    key: "{it.key}",
                                    alias: it.row.alias,
                                    display,
                                    input_per_1k: it.row.input_per_1k,
                                    output_per_1k: it.row.output_per_1k,
                                    multiplier: it.row.multiplier,
                                    index: idx,
                                    usable_groups: usable,
                                    alias_key: it.key,
                                    price_mode: mode,
                                    on_mode_change: move |m: PriceMode| on_mode_change.call((key_ref.clone(), m)),
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
