//! 实体设置页：分组 / 模型别名 / 渠道 三张可折叠卡片，各占一行。
//! 每张卡片上半是录入行，下半是该实体在拓扑里对应的节点内容。
//!
//! 数据在 `crate::state::EntityStore` 中（拓扑启动布局兜底快照）；
//! 写路径一律走 `crate::drawer_write` 的真实端点（分组/渠道 CRUD），
//! 成功后由调用方重拉 `network::load_network_data` 刷新画布——
//! #183 起画布不再由 store 行驱动，本地 store 行仅作启动布局兜底。

use dioxus::prelude::*;
use super::cards::{GroupsCard, AliasesCard};
use super::channels::ChannelsCard;

#[component]
pub fn EntitiesPanel() -> Element {
    let mut open = use_signal(|| [true, true, true]);

    rsx! {
        // 滚动由外层容器（拓扑抽屉）负责，这里别自带 overflow，
        // 否则锚点/滚动事件会对不上元素。
        div { class: "flex flex-col gap-3",
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

