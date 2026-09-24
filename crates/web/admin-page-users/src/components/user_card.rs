//! 单张用户卡的页面适配层：分组标签映射 + 操作区插槽。
//!
//! 卡片展示（AdminCard 三页签外壳、额度 CNY 换算、用量配色）复用 ui-components
//! 的共享 `ui::UserCard`；本文件只保留页面相关的两件事：
//! - 分组名 → 显示标签的 context 映射（取分组列表 remark，回落裸名，与弹窗
//!   chips 同口径）；
//! - 操作区交给 `UserActions`（`user_actions.rs`）装配，本组件不写任何
//!   按钮 rsx。
//!
//! 面板只经 props 拿 user 与三个回调，不感知卡片内部的展示推导。

use contract::api::admin::AdminUserDto;
use dioxus::prelude::*;

use super::user_actions::UserActions;

/// 单张用户卡（页面适配层）。
///
/// 【是什么】用户 tab 网格里的单张用户卡：把页面上下文（分组 remark 标签）与
/// 操作行（`UserActions`）组装成插槽，交给共享 `ui::UserCard` 渲染。
///
/// 【做什么】只做一件页面相关的事——分组名到展示标签的 context 映射；操作行
/// 由 `UserActions` 装配，卡片展示结构、额度换算、页签与外壳都在 `ui::UserCard`。
/// 本组件不发任何网络请求。
///
/// 【交互逻辑】卡片自身无交互；编辑 / 充值 / 启停由 `UserActions` 的三个回调
/// 抛回页面（语义见 `UserActions` 文档）。
///
/// 【样式】无自有样式：外壳与页签来自 `ui::UserCard`（共用 `AdminCard` 壳）；
/// 操作行 class 全部来自 `ui::ActionButtonGroup`（tone 着色，与旧卡逐字一致）。
///
/// 【子组件组成】`ui::UserCard`（共享卡片）、`UserActions`（操作行插槽）。
///
/// 【数据流】
/// - 对内(入)：`user`（单条 `AdminUserDto`）、`on_edit` / `on_topup` /
///   `on_toggle`（页面闭包，均以 `user.key` 定位行）。
/// - 对外(出)：三个 `EventHandler` 经 `UserActions` 抛回页面；`group_labels`
///   由本组件从 context 的分组信号推导后传入共享卡。
#[component]
pub fn UserCard(
    user: AdminUserDto,
    on_edit: EventHandler<String>,
    on_topup: EventHandler<String>,
    on_toggle: EventHandler<(String, String, Option<String>)>,
) -> Element {
    // 分组名 → 展示标签:取 context 分组列表的 remark,回落裸名(与弹窗 chips 同口径)。
    let groups_ctx = use_context::<Signal<Vec<(String, String)>>>();
    let group_labels: Vec<String> = user
        .groups
        .iter()
        .map(|name| {
            groups_ctx()
                .iter()
                .find(|(_, n)| n == name)
                .map(|(l, _)| l.clone())
                .unwrap_or_else(|| name.clone())
        })
        .collect();

    // DONE: 不再在组件函数里放内联 rsx 块,操作行抽成 `UserActions` 子组件再组合(R1.3 单 rsx) in=demo by=agent
    // 单内容 rsx:操作行整块交给 `UserActions`(按钮装配与下标分派都在那边)。
    // 槽值 Element 经槽内 rsx! 构造(dioxus 0.7 唯一方式,框架惯例例外)。
    rsx! {
        ui::UserCard {
            user: user.clone(),
            group_labels,
            actions: Some(rsx! {
                UserActions {
                    user: user.clone(),
                    on_edit,
                    on_topup,
                    on_toggle,
                }
            }),
        }
    }
}
