//! 单张用户卡的页面适配层：分组标签映射。
//!
//! 卡片展示（AdminCard 三页签外壳 + 底部通栏横线页签、额度 CNY 换算、用量配色、
//! 用户名 / 邮箱行内原地编辑）复用 ui-components 的共享 `ui::UserCard`；本文件
//! 只保留页面相关的一件事：分组名 → 展示标签的 context 映射（取分组列表 remark，
//! 回落裸名，与弹窗 chips 同口径）。
//!
//! 卡内操作按钮行已按维护者批注（2026-09-21）删除——启停 / 保存 / 删除将由卡牌
//! 外的图标按钮承担（布局设计待维护者确认后接入），届时在本文件重建按钮组并
//! 传回页面回调。

use contract::api::admin::AdminUserDto;
use dioxus::prelude::*;

/// 单张用户卡（页面适配层）。
///
/// 【是什么】用户 tab 网格里的单张用户卡：把页面上下文（分组 remark 标签）交给
/// 共享 `ui::UserCard` 渲染。
///
/// 【做什么】只做一件页面相关的事——分组名到展示标签的 context 映射；不负责卡片
/// 展示结构、额度换算、页签与外壳（都在 `ui::UserCard`），也不负责任何网络请求。
///
/// 【数据流】
/// - 对内(入)：`user`（单条 `AdminUserDto`）。
/// - 对外(出)：无（卡片零网络；行内编辑的草稿留卡内，保存入口待卡牌外按钮）。
#[component]
pub fn UserCard(user: AdminUserDto) -> Element {
    // 全部分组选项 (标签, 分组名)：取分组列表里的 remark 口径(与分组管理页同
    // 口径)，直接交给共享卡——卡内 chip 文案映射与 popover 全选项都由它驱动。
    let groups_ctx = use_context::<Signal<Vec<(String, String)>>>();

    rsx! {
        ui::UserCard {
            user,
            all_groups: groups_ctx(),
        }
    }
}
