//! 别名卡片网格区(编号段 3):标题计数 + 四态分支 + AliasCard 网格。
//!
//! 纯展示组件:数据(load/err/filtered/分组/通道开关)以值传入;
//! 交互通过 EventHandler 抛回页面。on_mode_change 收 (alias_key, mode) 二元组,
//! 对每张卡包装成单参闭包 —— 取代原先页面里的 make_mode_handler 闭包工厂。
//!
//! 卡片本体是 ui-components 的 `AliasCard`（四页签 + 共享外壳）；
//! 本文件只做四态分支与网格排版，不再自带卡片样式。
//!
//! 边界:筛选计算、可用分组判定之外的拉取/写回都在 `page.rs`(本文件只用
//! `usable_groups_for` 算这张卡的可用分组展示);卡片外壳 class 不在此定义,
//! 由 ui-components 的 `admin_card::shell` 统一。

use contract::api::admin::GroupDto;
use dioxus::prelude::*;
use ui::{CardGrid, DangerBlock, GhostButton, PlaceholderBlock, SectionHeader};

use super::shared::{
    AliasItem, BTN_RETRY, LBL_ALIAS_LIST, MSG_EMPTY, MSG_LOAD_FAILED, MSG_LOADING_LIST,
    OPT_BADGE_LOADING, PriceMode, SEC_LIST, usable_groups_for,
};

/// 别名卡片网格区:错误 / 加载 / 空 / 网格 四态。
///
/// 【是什么】别名 tab 的编号段 3:标题行 + 计数徽标 + 四态分支 + 别名卡片网格。
///
/// 【做什么】按 `err` / `loading` / `filtered` 的取值渲染四种形态之一,并在有数据时
/// 把每条 `AliasItem` 铺成 `ui::AliasCard`。不负责筛选(页面已把 filtered 算好)、
/// 不负责拉数据、不负责卡片内部的多页签逻辑(在 ui-components 的 `AliasCard` 里)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点错误态「重试」→ `on_retry` 抛回页面(MouseEvent,页面 `reload + 1` 触发重拉)。
/// - 在卡片上切换定价模式 → 卡片把 `PriceMode` 抛上来,本组件补上 `alias_key`
///   组成 `(String, PriceMode)` 再调 `on_mode_change`,页面据此写回 `rows`(纯本地状态,不发网络)。
/// - `on_edit` / `on_delete` 与四个通道开关 `c_*` 目前透传但未挂到卡片上
///   (卡片暂不渲染编辑入口),仅保留接线。
/// 数据交互:本组件自身**不发任何网络请求**。
///
/// 【样式】外壳 `section#aliases-sec-list` 为 `scroll-mt-8 space-y-4`;标题行由
/// `SectionHeader` 提供(左 `text-lg font-medium text-zinc-100` 标题 + 右
/// `rounded-full bg-zinc-800` 计数胶囊);错误态用红底 `DangerBlock`,加载/空态用
/// 虚线描边 `PlaceholderBlock`;网格为 `grid grid-cols-1 gap-3 md:grid-cols-3
/// lg:grid-cols-5`(手机 1 / 中屏 3 / 大屏 5 列)。
///
/// 【子组件组成】`SectionHeader`(标题 + 计数)、`DangerBlock`(错误块)、
/// `PlaceholderBlock`(加载/空态)、`GhostButton`(重试)、`CardGrid`(网格容器)、
/// `ui::AliasCard`(单张别名卡)。
///
/// 【数据流】
/// - 对内(入):`loading` / `err`(页面 effect 的加载与失败态,err 优先于 loading 渲染)、
///   `filtered`(页面按 search + filter_tier 筛好的 `(原始下标, AliasItem)`,只用于
///   计数与渲染,下标不参与定位)、`groups`(`page` effect 拉到的分组,供
///   `usable_groups_for` 算每卡可用分组与倍率)、`c_output_on` 等四个通道开关
///   (与弹窗共享,页面持有)、`on_edit` / `on_delete` / `on_retry`。
/// - 对外(出):`on_mode_change((alias_key, mode))` → 页面 `on_mode_change` 就地改写
///   `rows` 中该条的 `price_mode`;`on_retry` → 页面重拉;`on_edit` / `on_delete`
///   为预留出口(分别指向页面 `open_edit` / `write_delete`)。
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
        OPT_BADGE_LOADING.to_string()
    } else {
        format!("{} 个", filtered.len())
    };

    rsx! {
        section { id: "aliases-sec-list", class: "scroll-mt-8 space-y-4",
            SectionHeader { title: SEC_LIST, badge: badge }

            if let Some(e) = err {
                DangerBlock {
                    title: MSG_LOAD_FAILED,
                    detail: e,
                    GhostButton { label: BTN_RETRY, onclick: on_retry }
                }
            } else if loading {
                PlaceholderBlock { message: MSG_LOADING_LIST }
            } else if filtered.is_empty() {
                PlaceholderBlock { message: MSG_EMPTY }
            } else {
                CardGrid { aria_label: LBL_ALIAS_LIST, testid: "aliases-list",
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
