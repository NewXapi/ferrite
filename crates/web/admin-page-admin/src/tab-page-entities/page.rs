//! 实体设置页：分组 / 模型别名 / 渠道 三张可折叠卡片，各占一行。
//! 每张卡片上半是录入行，下半是该实体在拓扑里对应的节点内容。
//!
//! 数据在 `crate::state::EntityStore` 中（拓扑启动布局兜底快照）；
//! 写路径一律走 `crate::drawer_write` 的真实端点（分组/渠道 CRUD），
//! 成功后由调用方重拉 `network::load_network_data` 刷新画布——
//! #183 起画布不再由 store 行驱动，本地 store 行仅作启动布局兜底。
//!
//! 边框:本文件只持有三张卡的展开态并做薄组装;三张卡的业务逻辑分别在
//! `cards.rs`(分组/别名)与 `channels.rs`(渠道)。

use super::cards::{AliasesCard, GroupsCard};
use super::channels::ChannelsCard;
use dioxus::prelude::*;

/// 实体设置面板：三张可折叠实体卡的薄组装。
///
/// 【是什么】实体设置页的入口组件,把分组 / 模型别名 / 渠道三张卡竖排组装成面板。
///
/// 【做什么】只持有一个长度 3 的展开态数组并把它按下标分发给三张卡;不负责任何
/// 业务数据(在 `EntityStore`)与写路径(在各卡内调 `drawer_write`)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点某张卡的头部 → 对应 `on_toggle` 翻转 `open` 数组里该下标(纯本地状态),
///   三张卡相互独立,不做手风琴互斥。
/// 本组件自身**不发任何网络请求**。
///
/// 【样式】外层 `div.flex flex-col gap-3`;卡片自身的边框与头部样式封在
/// `CardPanel` 里。顶层刻意不带 `overflow`,滚动交给外层拓扑抽屉容器,
/// 否则锚点/滚动事件会对不上元素。
///
/// 【子组件组成】`GroupsCard` / `AliasesCard` / `ChannelsCard`(均来自本目录)。
///
/// 【数据流】
/// - 对内(入):无 prop —— 面板不接收外部参数;三张卡各自从
///   `use_context::<EntityStore>()` 取数据。
/// - 对外(出):把 `open[i]` 与翻转闭包交给各卡;卡片内部的写回由卡片自己完成。
#[component]
pub fn EntitiesPanel() -> Element {
    // 三张卡的展开态各占一位(顺序 = 分组 / 别名 / 渠道),按位下发给各卡。
    let mut open = use_signal(|| [true, true, true]);

    rsx! {
        // 滚动由外层容器（拓扑抽屉）负责，这里别自带 overflow，
        // 否则锚点/滚动事件会对不上元素。
        div { class: "flex flex-col gap-3",
            // 三张可折叠实体卡:各占一行,上半录入行 + 下半拓扑节点内容。
            // open 数组在页面持有(手风琴互斥与否由页面决定);卡片写路径走
            // drawer_write 真实端点,成功后 bump_topo_refresh 刷拓扑画布。
            GroupsCard {
                open: open()[0],
                on_toggle: move |_| { let mut o = open(); o[0] = !o[0]; open.set(o); },
            }
            AliasesCard {
                open: open()[1],
                on_toggle: move |_| { let mut o = open(); o[1] = !o[1]; open.set(o); },
            }
            ChannelsCard {
                open: open()[2],
                on_toggle: move |_| { let mut o = open(); o[2] = !o[2]; open.set(o); },
            }
        }
    }
}
