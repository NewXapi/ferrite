//! 单张用户卡的页面适配层：分组标签映射 + 操作区插槽。
//!
//! 卡片展示（AdminCard 三页签外壳、额度 CNY 换算、用量配色）复用 ui-components
//! 的共享 `ui::UserCard`；本文件只保留页面相关的两件事：
//! - 分组名 → 展示标签的 context 映射（取分组列表 remark，回落裸名，与弹窗
//!   chips 同口径）；
//! - 编辑 / 充值 / 启停三个操作按钮（回调语义与 wire 动作名逐字保留旧卡）。
//!
//! 面板只经 props 拿 user 与三个回调，不感知卡片内部的展示推导。

use contract::api::admin::AdminUserDto;
use dioxus::prelude::*;

use super::shared::{BTN_EDIT, BTN_TOPUP, STATUS_DISABLED, STATUS_ENABLED};

/// 单张用户卡（页面适配层）。
///
/// 【是什么】用户 tab 网格里的单张用户卡：把页面上下文（分组 remark 标签）与
/// 三个操作按钮组装成插槽，交给共享 `ui::UserCard` 渲染。
///
/// 【做什么】只做两件页面相关的事——分组名到展示标签的 context 映射、操作区
/// 按钮组；不负责卡片展示结构、额度换算、页签与外壳（都在 `ui::UserCard`），
/// 也不负责任何网络请求。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互：
/// - 点「编辑」→ `on_edit(key)`，页面开新建/编辑弹窗。
/// - 点「充值」→ `on_topup(key)`，页面开充值弹窗。
/// - 点「启用/停用」→ `on_toggle((key, action, None))`，`action` 是 snake_case
///   的 `enable` / `disable`（后端 `ManageUserAction` 拒中文变体），页面据此
///   调 `POST /api/user/users/manage`。
/// 本组件自身**不发任何网络请求**。
///
/// 【样式】无自有样式：外壳与页签来自 `ui::UserCard`（共用 `AdminCard` 壳）；
/// 操作行 `mt-4 flex gap-1.5 border-t border-zinc-800 pt-3` + 三个等宽按钮，
/// class 与旧卡逐字一致。
///
/// 【子组件组成】`ui::UserCard`（共享卡片，含 `AdminCard` 外壳与圆点页签）。
///
/// 【数据流】
/// - 对内(入)：`user`（单条 `AdminUserDto`）、`on_edit` / `on_topup` /
///   `on_toggle`（页面闭包，均以 `user.key` 定位行）。
/// - 对外(出)：三个 `EventHandler` 把用户意图抛回页面；`group_labels` 由本
///   组件从 context 的分组信号推导后传入共享卡。
#[component]
pub fn UserCard(
    user: AdminUserDto,
    on_edit: EventHandler<String>,
    on_topup: EventHandler<String>,
    on_toggle: EventHandler<(String, String, Option<String>)>,
) -> Element {
    // 分组名 → 展示标签:取分组列表里的 remark(与分组管理页同口径),
    // 取不到回落裸名 —— 卡片与弹窗 chips 必须显示同一套文案
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

    // 三个回调各自持有 key 的副本(EventHandler 是 move 捕获,String 不可 Copy)。
    let edit_key = user.key.clone();
    let topup_key = user.key.clone();
    let toggle_key = user.key.clone();

    // 操作区插槽：按钮 class / testid / 回调逐字保留旧卡，交给共享卡渲染在
    // 基本信息页签底部。
    let actions = rsx! {
        div { class: "mt-4 flex gap-1.5 border-t border-zinc-800 pt-3",
            button {
                class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white",
                "data-testid": "user-edit",
                onclick: move |_| on_edit.call(edit_key.clone()),
                "{BTN_EDIT}"
            }
            button {
                class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-emerald-400 transition-colors hover:bg-zinc-700 hover:text-emerald-300",
                "data-testid": "user-topup",
                onclick: move |_| on_topup.call(topup_key.clone()),
                "{BTN_TOPUP}"
            }
            button {
                class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-amber-400 transition-colors hover:bg-zinc-700 hover:text-amber-300",
                "data-testid": "user-toggle",
                onclick: move |_| on_toggle.call((
                    toggle_key.clone(),
                    // wire 动作名是 snake_case 英文(后端 ManageUserAction 拒中文变体);
                    // 按钮文案仍显示中文,仅 value 走 enable/disable
                    if user.status == 1 { "disable".to_string() } else { "enable".to_string() },
                    None,
                )),
                if user.status == 1 { {STATUS_DISABLED} } else { {STATUS_ENABLED} }
            }
        }
    };

    rsx! {
        ui::UserCard {
            user,
            group_labels,
            actions: Some(actions),
        }
    }
}
