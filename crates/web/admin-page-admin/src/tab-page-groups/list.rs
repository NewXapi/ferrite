//! 分组卡片网格区(编号段 3):标题计数 + 四态分支(loading / error / empty / data)
//! + `GroupCard` 网格。
//!
//! 纯展示组件:数据与写回回调由 page 注入,自身零 `use_signal`、不发网络请求;
//! 卡片独有的交互(滑条拖拽、按钮组)全在 `modal.rs` 的 `GroupCard` 内。
//!
//! 删 / 启停 / 倍率三类卡片操作统一走 `on_write` 回传 `(key, WriteOp)`,
//! 由页面分派到对应写工厂;编辑走 `on_edit` 回传 key。
//!
//! 边界:筛选计算、可用分组判定、拉取/写回都在 `page.rs`;卡片外壳与倍率滑条样式
//! 不在本文件定义,由 `modal.rs::GroupCard` 承载。

use dioxus::prelude::*;

use contract::api::admin::GroupDto;

use super::modal::GroupCard;
use super::shared::{
    BTN_RETRY, MSG_EMPTY, MSG_LOAD_FAILED, MSG_LOADING_LIST, OPT_BADGE_LOADING, SEC_LIST, WriteOp,
};

/// 分组列表(四态 + 卡片网格)。
///
/// 【是什么】分组 tab 的编号段 3:标题行 + 计数徽标 + 四态分支,有数据时先渲染一张
/// 只读原型卡示例,再把每条 `GroupDto` 铺成可操作的 `GroupCard`。
///
/// 【做什么】按 `err` / `loading` / `filtered` 渲染四态;每张卡把编辑 / 删除 / 启停 /
/// 倍率四类操作包成对应回调。不负责筛选(`filtered` 由页面算好)、不负责拉数据、
/// 不负责写回网络(只抛 `WriteOp`)、不负责卡片内部样式。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点错误态「重试」→ `on_retry` 抛回页面,页面 `reload + 1` 触发重拉。
/// - 点卡片「编辑」→ `on_edit(key)`,页面 `open_edit` 回填表单并置 `ModalState::Edit`,打开弹窗。
/// - 点卡片「删除」→ `on_write((key, WriteOp::Delete))`,页面调 `delete_group_api` 并整页重拉。
/// - 点卡片「启用/停用」→ `on_write((key, WriteOp::ToggleStatus(target)))`,target 由当前
///   status 取反(1↔2),页面调 `set_group_status_api` 后就地改本地 `status`(不重拉)。
/// - 拖动倍率滑条松手 → `on_write((key, WriteOp::SetRatio(v)))`,页面调
///   `update_group_ratio_api` 后就地改本地 `ratio`。
/// 数据交互:本组件自身**不发任何网络请求**;回调最终触发的写请求都在 `page.rs`。
///
/// 【样式】外壳 `section#groups-sec-list` 为 `scroll-mt-8 space-y-4`;标题行左侧
/// `text-lg font-medium text-zinc-100`,右侧计数胶囊 `rounded-full bg-zinc-800`
/// (加载时显示 `OPT_BADGE_LOADING`,否则 `N 组`);错误态红底
/// `rounded-2xl border border-red-800/60 bg-red-950/40 py-10`;加载/空态为虚线描边
/// `rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16`;示例区与
/// 网格均为 `grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5`(手机 1 / 中屏 3 / 大屏 5 列)。
///
/// 【子组件组成】`modal::GroupCard`(可操作分组卡,外壳用共用样式壳
/// `ui::CardShell`);四态块为原生 `div` / `p` / `button`。
///
/// 【数据流】
/// - 对内(入):`filtered`(页面按关键词 + 分级筛好的 `GroupDto`,用于计数与渲染)、
///   `loading` / `err`(页面 effect 的加载与失败态;`err` 优先于 `loading` 渲染)、
///   `on_edit` / `on_write` / `on_retry`(均为页面闭包)。
/// - 对外(出):`on_edit(key)` → 页面 `open_edit` 打开编辑弹窗;`on_write((key, op))` →
///   页面 `on_write` 按 `WriteOp` 分派到 `write_delete` / `write_toggle`,落到
///   delete / set_status / set_ratio 三个 API;`on_retry` → 页面 `reload + 1`。
#[component]
pub fn GroupsList(
    filtered: Vec<GroupDto>,
    loading: bool,
    err: Option<String>,
    on_edit: EventHandler<String>,
    on_write: EventHandler<(String, WriteOp)>,
    on_retry: EventHandler<()>,
) -> Element {
    // 分页：列表内部 UI 状态（不跨组件）；筛选后条数变小时 clamp 到最后一页，
    // 不落空页（详见 aliases list 同款注释）。
    let mut page = use_signal(|| 0usize);
    let visible = ui::page_slice(&filtered, page(), ui::CARD_PAGE_SIZE).to_vec();

    rsx! {
        section { id: "groups-sec-list", class: "scroll-mt-8 space-y-4",
            ui::SectionHeader {
                title: SEC_LIST.to_string(),
                badge: if loading { OPT_BADGE_LOADING.to_string() } else { format!("{} 组", filtered.len()) },
                trailing: rsx! {
                    ui::Pager {
                        total: filtered.len(),
                        page,
                        on_change: move |p| page.set(p),
                        testid: "groups-pager",
                    }
                },
            }

            if let Some(e) = err {
                div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 py-10 text-center",
                    p { class: "text-sm {ui::C_DANGER}", "{MSG_LOAD_FAILED}" }
                    p { class: "mt-1 text-xs {ui::C_DANGER}", "{e}" }
                    button {
                        class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 {ui::TYPE_DESC} hover:bg-zinc-800",
                        "data-testid": "retry-groups",
                        onclick: move |_| on_retry.call(()),
                        "{BTN_RETRY}"
                    }
                }
            } else if loading {
                div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "{ui::C_MUTED}", "{MSG_LOADING_LIST}" }
                }
            } else if filtered.is_empty() {
                div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "{ui::C_MUTED}", "{MSG_EMPTY}" }
                }
            } else {
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                    "data-testid": "groups-list",
                    for g in visible {
                        {
                            let edit_key = g.key.clone();
                            let delete_key = g.key.clone();
                            let toggle_key = g.key.clone();
                            let ratio_key = g.key.clone();
                            let current_status = g.status;
                            let is_default = g.name == "default";
                            // 启用/停用:按当前 status 取目标值 (1↔2);
                            // target 在 group 值移入 rsx 前算好, 避免 move 后再借用
                            let toggle_target = if current_status == 1 { 2 } else { 1 };
                            let on_toggle_status = move |_| {
                                on_write.call((
                                    toggle_key.clone(),
                                    WriteOp::ToggleStatus(toggle_target),
                                ));
                            };
                            // 倍率滑条松手写回: 复用同一写工厂, 就地更新本地 ratio
                            let on_ratio_drag = move |v: f64| {
                                on_write.call((ratio_key.clone(), WriteOp::SetRatio(v)));
                            };
                            rsx! {
                                GroupCard {
                                    key: "{g.key}",
                                    group: g,
                                    is_default,
                                    on_edit: move |_| on_edit.call(edit_key.clone()),
                                    on_delete: move |_| {
                                        on_write.call((delete_key.clone(), WriteOp::Delete))
                                    },
                                    on_toggle_status,
                                    on_ratio_drag,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
