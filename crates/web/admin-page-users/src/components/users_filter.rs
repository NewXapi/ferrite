//! 筛选区组件:搜索框 + 分组/状态/角色三段胶囊 + 刷新/新建按钮。
//! 从 tab-page/users.rs 段 2 沉降(R1.1 粒度下限 = 一个编号段落)。

use dioxus::prelude::*;
use ui::SegmentedCapsule;

use crate::shared::{BTN_NEW_USER, BTN_REFRESH, MSG_SEARCH_HINT, SEC_FILTER};

/// 筛选区(页面段落 2)。
///
/// 【是什么】用户列表的筛选栏:搜索输入框、分组/状态/角色三个胶囊分段、刷新与新建按钮。
///
/// 【做什么】渲染筛选项并回写四个筛选信号;按钮把「刷新」「新建用户」意图抛回
/// 页面。不负责按筛选条件过滤列表(页面层派生 `filtered`),不负责拉分组列表。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 输入搜索词 → `search.set(value)`(页面据此过滤)。
/// - 点胶囊分段 → 对应 `*_idx.set(i)`。
/// - 点「刷新」→ `on_refresh(())`,页面 `reload + 1` 重拉列表。
/// - 点「新建用户」→ `on_new(())`,页面打开新建弹窗。
/// 本组件自身**不发任何网络请求**(分组列表由页面拉取后以 `filter_groups` 传入)。
///
/// 【样式】`section#users-sec-filter` 卡片壳 `rounded-xl border-zinc-800 bg-zinc-900 p-5`;
/// 标题行 + 两按钮;搜索框 `rounded-xl border-zinc-700/80 bg-zinc-950`;
/// 手机端胶囊每行最多 3 段(class 与原页面逐字一致)。
///
/// 【子组件组成】`ui::SegmentedCapsule` ×3 + 原生 input/button。
///
/// 【数据流】
/// - 对内(入):`filter_groups`(全部 + 真实分组,页面派生)、`statuses`/`roles`
///   (静态枚举,页面常量)、`search`/`group_idx`/`status_idx`/`role_idx` 四个筛选信号。
/// - 对外(出):`on_refresh` / `on_new` 两个用户意图。
#[component]
pub fn UsersFilterSection(
    filter_groups: Vec<(String, String)>,
    statuses: &'static [(&'static str, u8)],
    roles: &'static [(&'static str, u16)],
    search: Signal<String>,
    group_idx: Signal<usize>,
    status_idx: Signal<usize>,
    role_idx: Signal<usize>,
    on_refresh: EventHandler<()>,
    on_new: EventHandler<()>,
) -> Element {
    rsx! {
        section {
            id: "users-sec-filter",
            class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
            div { class: "flex items-center justify-between gap-3",
                h2 { class: "text-sm font-medium text-zinc-300", "{SEC_FILTER}" }
                div { class: "flex gap-2",
                    button {
                        class: "shrink-0 rounded-xl border border-zinc-700 px-3 py-2 text-xs text-zinc-300 transition-colors hover:bg-zinc-800",
                        "data-testid": "refresh-users",
                        onclick: move |_| on_refresh.call(()),
                        "{BTN_REFRESH}"
                    }
                    button {
                        class: "shrink-0 rounded-xl bg-white px-4 py-2 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                        "data-testid": "new-user",
                        onclick: move |_| on_new.call(()),
                        "{BTN_NEW_USER}"
                    }
                }
            }

            input {
                class: "w-full rounded-xl border border-zinc-700/80 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 outline-none transition focus:border-zinc-500",
                r#type: "text",
                placeholder: MSG_SEARCH_HINT,
                "data-testid": "users-search",
                value: "{search}",
                oninput: move |e| search.set(e.value()),
            }

            // 分组 / 状态 / 角色:胶囊分段,手机每行最多 3 段
            div { class: "flex flex-col gap-3",
                SegmentedCapsule {
                    items: filter_groups.iter().map(|(l, _)| l.clone()).collect(),
                    active: group_idx(),
                    on_select: move |i: usize| group_idx.set(i),
                }
                SegmentedCapsule {
                    items: statuses.iter().map(|(l, _)| l.to_string()).collect(),
                    active: status_idx(),
                    on_select: move |i: usize| status_idx.set(i),
                }
                SegmentedCapsule {
                    items: roles.iter().map(|(l, _)| l.to_string()).collect(),
                    active: role_idx(),
                    on_select: move |i: usize| role_idx.set(i),
                }
            }
        }
    }
}
