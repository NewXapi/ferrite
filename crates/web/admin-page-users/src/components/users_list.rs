//! 用户列表区组件:分页 + 四态分支(错误 / 加载 / 空 / 卡片网格)。
//! 从 tab-page/users.rs 段 3 沉降(R1.1 粒度下限 = 一个编号段落)。
//! 标题行(区标题 + 徽标 + 分页器)收敛在 `ListHeader`,每个组件函数至多一个 rsx。

use contract::api::admin::AdminUserDto;
use dioxus::prelude::*;
use ui::{CARD_PAGE_SIZE, CardGrid, Pager, SectionHeader, page_slice};

use crate::components::user_card::UserCard;
use crate::shared::{
    BTN_RETRY, LBL_PERSON, MSG_LOAD_USERS_FAIL, MSG_LOADING, MSG_LOADING_USERS, MSG_NO_MATCH,
    SEC_LIST,
};

/// 列表区标题行:区标题 + 条数/加载徽标 + 右位分页器(`ui::Pager`)。
///
/// 【是什么】列表段落的第一行:左标题与徽标,右分页游标。
///
/// 【做什么】把「一对多」的翻页收在 `ui::Pager` 条目组件里;分页信号由列表
/// 组件持有,这里只读只写。
///
/// 【交互逻辑】点分页器 → `page.set(p)`,父列表按新页重渲。
///
/// 【样式】`ui::SectionHeader` 标题行 + `ui::Pager`(class 与旧页逐字一致)。
///
/// 【子组件组成】`ui::SectionHeader`、`ui::Pager`。
///
/// 【数据流】
/// - 对内(入):`badge`(条数/加载文案)、`total` / `page`(分页游标)。
/// - 对外(出):无(翻页只改传入的 `page` 信号)。
#[component]
pub fn ListHeader(badge: String, total: usize, page: Signal<usize>) -> Element {
    // 分页器经 SectionHeader 的 trailing 槽注入:槽值是 Element,
    // dioxus 0.7 下唯一构造方式 = 槽内一层 rsx!(组件字面量 + .into() 不通,
    // E0559 completions 变体)。槽内 rsx 是框架惯例例外,不算第二个内容 rsx。
    rsx! {
        SectionHeader {
            title: SEC_LIST.to_string(),
            badge,
            trailing: rsx! {
                Pager {
                    total,
                    page,
                    on_change: move |p| page.set(p),
                    testid: "users-pager".to_string(),
                }
            },
        }
    }
}
/// 用户列表区(页面段落 3)。
///
/// 【是什么】筛选后的用户卡片网格:标题行(条数徽标 + 分页器)+ 四态分支。
///
/// 【做什么】把传入的筛选结果分页渲染;分页游标是列表内部 UI 状态,按 R2
/// 沉降在本组件。不负责筛选(页面层派生 `filtered`)、不负责拉取与写操作。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 翻页 → 内部 `page.set(p)`(不跨组件)。
/// - 重试(错误态)→ `on_retry(())`,页面 `reload + 1` 重拉。
/// - 卡片操作(编辑 / 充值 / 启停)→ 原样透传 `on_edit` / `on_topup` / `on_toggle`。
/// 本组件自身**不发任何网络请求**。
///
/// 【样式】`section#users-sec-list.space-y-4`;错误态红壳卡 / 加载与空态虚线卡 /
/// `CardGrid` 网格(class 与原页面逐字一致)。
///
/// 【子组件组成】`ListHeader`(标题行 + 分页器)、`ui::CardGrid` +
/// `components::UserCard`(每用户一张)。
///
/// 【数据流】
/// - 对内(入):`filtered`(页面派生的筛选结果)、`loading`/`err` 拉取状态、
///   `on_retry`/`on_edit`/`on_topup`/`on_toggle` 四个意图回调。
/// - 对外(出):分页与卡片操作全部经 EventHandler 抛回页面。
#[component]
pub fn UsersListSection(
    filtered: Vec<AdminUserDto>,
    loading: Signal<bool>,
    err: Signal<Option<String>>,
    on_retry: EventHandler<()>,
    on_edit: EventHandler<String>,
    on_topup: EventHandler<String>,
    on_toggle: EventHandler<(String, String, Option<String>)>,
) -> Element {
    // 分页:列表内部 UI 状态(不跨组件)——按 R2 沉降在本组件;
    // 筛选后条数变小时 clamp 到最后一页,不落空页(与 admin-page-admin 各列表同款策略)。
    let page = use_signal(|| 0usize);
    let visible = page_slice(&filtered, page(), CARD_PAGE_SIZE).to_vec();
    let badge = if loading() {
        MSG_LOADING.to_string()
    } else {
        format!("{} {LBL_PERSON}", filtered.len())
    };

    rsx! {
        section { id: "users-sec-list", class: "scroll-mt-8 space-y-4",
            ListHeader {
                badge,
                total: filtered.len(),
                page,
            }

            if let Some(e) = err() {
                div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 py-10 text-center",
                    p { class: "{ui::T_text_sm} {ui::T_text_red_300}", "{MSG_LOAD_USERS_FAIL}" }
                    p { class: "mt-1 {ui::T_text_xs} text-red-400/70", "{e}" }
                    button {
                        class: "mt-3 rounded-xl border {ui::T_border_zinc_700} px-3 py-1.5 {ui::T_text_xs} {ui::T_text_zinc_300} hover:{ui::T_bg_zinc_800}",
                        onclick: move |_| on_retry.call(()),
                        "{BTN_RETRY}"
                    }
                }
            } else if loading() {
                div { class: "rounded-2xl border border-dashed {ui::T_border_zinc_700} bg-zinc-900/50 py-16 text-center",
                    p { class: "{ui::T_text_zinc_400}", "{MSG_LOADING_USERS}" }
                }
            } else if filtered.is_empty() {
                div { class: "rounded-2xl border border-dashed {ui::T_border_zinc_700} bg-zinc-900/50 py-16 text-center",
                    p { class: "{ui::T_text_zinc_400}", "{MSG_NO_MATCH}" }
                }
            } else {
                CardGrid { aria_label: SEC_LIST.to_string(), testid: "users-list".to_string(),
                    for user in visible {
                        UserCard {
                            key: "{user.key}",
                            user,
                            on_edit,
                            on_topup,
                            on_toggle,
                        }
                    }
                }
            }
        }
    }
}
