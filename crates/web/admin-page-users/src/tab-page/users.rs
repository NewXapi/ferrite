//! 用户管理面板 —— 统计 / 筛选 / 用户卡片三区,组合式编排。
//!
//! 本文件只放「状态 + 拉取 effect + 区段组件组合」(spec R1: 页面 rsx 元素嵌套 ≤1 层):
//! - 区段: `components::{UsersStatsSection, UsersFilterSection, UsersListSection}`
//! - 浮层: `components::{UserForm, TopUpForm}`(字段信号已按 R2 沉入 UserForm,预填走 user prop)
//! - 文案常量在根级 `shared`,格式化助手在根级 `format`
//!   (`used_pct` / `short_key` 是与 admin-page-account 同口径的跨页语义,阶段 1 上提 ui-components)。
//!
//! 数据来自真实后端:挂载时 `use_effect` 拉 `list_users_api` 写入 `users` signal;
//! 筛选项(搜索 / 分组 / 状态 / 角色)在前端对返回列表过滤,与 mock 时期的 UX 一致。
//! enable/disable 与充值走 `manage_user_api`,新建走 `create_user_api`
//! (R3: 闭包只做写状态与调用提取函数 `create_user`)。

use client::ApiClient;
use contract::api::admin::{AdminUserDto, ManageUserRequest};
use dioxus::prelude::*;

use crate::api::{self, current_month_prefix, list_users_api, manage_user_api};
use crate::components::topup_form::TopUpForm;
use crate::components::user_form::UserForm;
use crate::components::users_filter::UsersFilterSection;
use crate::components::users_list::UsersListSection;
use crate::components::users_stats::UsersStatsSection;
use crate::format::fmt_cny;
use crate::shared::{
    LBL_CONSUMED_TOTAL, LBL_ENABLED_USERS, LBL_GRANTED_TOTAL, LBL_NEW_THIS_MONTH, LBL_TOTAL_USERS,
    MSG_ACTION_ERR, MSG_ACTION_OK, MSG_CREATE_ERR, MSG_USER_CREATED, OPT_ALL,
};

/// 弹窗状态:关闭 / 新建 / 编辑某用户(key 标识)
#[derive(Clone, PartialEq)]
enum Form {
    Closed,
    New,
    Edit(String),
}

/// R3 提取:新建用户(原 on_create 闭包内联的 spawn 逻辑)。
/// 成功 → 成功通知 + reload 自增;失败 → 失败前缀拼错误详情;busy 驱动通知条进行中态。
fn create_user(
    mut notice: Signal<Option<String>>,
    mut reload: Signal<u32>,
    mut busy: Signal<bool>,
    req: api::CreateUserRequest,
) {
    notice.set(None);
    busy.set(true);
    spawn(async move {
        let client = ApiClient::shared().clone();
        match api::create_user_api(&client, &req).await {
            Ok(_) => {
                notice.set(Some(MSG_USER_CREATED.to_string()));
                reload.set(reload() + 1);
            }
            Err(e) => notice.set(Some(format!("{MSG_CREATE_ERR}:{e}"))),
        }
        busy.set(false);
    });
}

#[component]
pub fn UsersPanel() -> Element {
    let search = use_signal(String::new);
    let group_idx = use_signal(|| 0usize);
    let status_idx = use_signal(|| 0usize);
    let role_idx = use_signal(|| 0usize);

    let mut form = use_signal(|| Form::Closed);
    let mut topup = use_signal(|| None::<String>);

    // 真实数据 + 加载/错误态
    let mut users = use_signal(Vec::<AdminUserDto>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    // 写操作进行中 / 成功提示
    let busy = use_signal(|| false);
    let notice = use_signal(|| None::<String>);
    // reload 计数:触发一次即重拉列表(写操作后刷新)
    let mut reload = use_signal(|| 0u32);

    // 真实分组列表(GET /api/group):筛选胶囊与弹窗 chips 共用。
    // 提供为 context,让 GroupChips 不经 props 也能取到同一份列表。
    let mut groups = use_signal(Vec::<(String, String)>::new);
    let _ = use_context_provider(|| groups);
    use_effect(move || {
        spawn(async move {
            let client = ApiClient::shared().clone();
            if let Ok(list) = api::list_groups_api(&client).await {
                groups.set(list);
            }
        });
    });

    // 拉取触发依赖:仅 reload 计数(挂载 + 写操作成功后的刷新)。
    // 列表长度必须 `peek()`:`.read()` 会让本 effect 订阅自己即将写入的 `users`
    // signal,每条响应 set 都重跑 effect → 重拉死循环(对齐 channels.rs 同款 effect)。
    use_effect(move || {
        let _ = reload();
        // stale-while-revalidate:只有首次(列表为空)才显示「加载中」占位符;
        // 写操作(禁用/充值/编辑)触发的刷新保留旧列表原地更新,不整片闪掉。
        // 闪烁根因:loading=true 会把已渲染的卡片网格换成占位符,拉完再换回。
        if users.peek().is_empty() {
            loading.set(true);
        }
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            // size=100:后端默认 20 会静默截断,统计卡「总用户」会少算
            match list_users_api(&client, None, Some(1), Some(100)).await {
                Ok(page) => {
                    users.set(page.items);
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    // 筛选项:分组走真实列表 + 首项「全部」;状态/角色仍是固定枚举。
    // filter_groups 同源两用:本层匹配 + UsersFilterSection 展示,故派生留在本层。
    let filter_groups = {
        let mut v = vec![(OPT_ALL.to_string(), String::new())];
        v.extend(groups().clone());
        v
    };
    let statuses = api::fetch_statuses();
    let roles = api::fetch_roles();
    let all = users();

    let total = all.len();
    let enabled = all.iter().filter(|u| u.status == 1).count();
    let new_this_month = all
        .iter()
        .filter(|u| u.created_at.starts_with(&current_month_prefix()))
        .count();
    let granted: i64 = all.iter().map(|u| u.quota).sum();
    let consumed: i64 = all.iter().map(|u| u.used_quota).sum();

    let stats: [(String, &'static str); 5] = [
        (total.to_string(), LBL_TOTAL_USERS),
        (enabled.to_string(), LBL_ENABLED_USERS),
        (new_this_month.to_string(), LBL_NEW_THIS_MONTH),
        (fmt_cny(granted), LBL_GRANTED_TOTAL),
        (fmt_cny(consumed), LBL_CONSUMED_TOTAL),
    ];

    let filtered: Vec<AdminUserDto> = {
        let q = search().trim().to_lowercase();
        let want_group = &filter_groups[group_idx()].1;
        let want_status = statuses[status_idx()].1;
        let want_role = roles[role_idx()].1;
        all.iter()
            .filter(|u| {
                if !q.is_empty()
                    && !u.username.to_lowercase().contains(&q)
                    && !u.email.to_lowercase().contains(&q)
                {
                    return false;
                }
                if !want_group.is_empty() && !u.groups.contains(want_group) {
                    return false;
                }
                if want_status != 0 && u.status != want_status {
                    return false;
                }
                if want_role != 0 && u.role != want_role {
                    return false;
                }
                true
            })
            .cloned()
            .collect()
    };

    // 打开新建/编辑弹窗时预填字段:字段信号已按 R2 沉入 UserForm,
    // 预填改为挂载时按 user prop 注水,此处只剩状态机转移。
    let open_edit = move |key: String| {
        // 与旧实现等价:key 必须仍存在于列表才开编辑弹窗
        if users().iter().any(|u| u.key == key) {
            form.set(Form::Edit(key));
        }
    };

    // 写操作助手工厂:返回独立闭包,分别交给 UserCard(启用/禁用)、
    // UserForm(编辑回写)、TopUpForm(充值)。Signal 是 Copy;每次调用先复制一份
    // 再 move 进 async,避免把闭包捕获的 signal 移动出去(FnMut 不允许)。
    let make_manage = || {
        let captured = (busy, notice, reload);
        move |key: String, action: String, value: Option<String>| {
            let (mut b, mut n, mut r) = captured;
            spawn(async move {
                b.set(true);
                n.set(None);
                let client = ApiClient::shared().clone();
                let req = ManageUserRequest { key, action, value };
                match manage_user_api(&client, &req).await {
                    Ok(_) => {
                        n.set(Some(MSG_ACTION_OK.to_string()));
                        r.set(r() + 1);
                    }
                    Err(e) => n.set(Some(format!("{MSG_ACTION_ERR}:{e}"))),
                }
                b.set(false);
            });
        }
    };
    let manage_toggle = make_manage();
    let manage_form = make_manage();
    let manage_topup = make_manage();

    // 弹窗编辑态:从 form() 派生,放在 rsx! 之外(宏内不允许 let 语句)
    let editing = matches!(form(), Form::Edit(_));
    let edit_key = match form() {
        Form::Edit(k) => Some(k),
        _ => None,
    };
    // R2 预填源:编辑态查到的用户(None = 新建态),UserForm 挂载时按此注水字段信号
    let edit_user = edit_key
        .as_ref()
        .and_then(|k| all.iter().find(|u| &u.key == k))
        .cloned();
    // 充值弹窗的当前额度:同样放在 rsx! 之外计算
    let topup_quota = topup()
        .and_then(|k| users().iter().find(|u| u.key == k).map(|u| u.quota))
        .unwrap_or(0);

    rsx! {
        div { class: "flex flex-col gap-6", "data-testid": "users-panel",

            // 通知条(成功/错误/进行中)—— 1 层布局包装,豁免 R1
            if let Some(msg) = notice() {
                div { class: "rounded-xl border {ui::T_border_zinc_700} {ui::T_bg_zinc_900} px-4 py-2 {ui::T_text_xs} {ui::T_text_zinc_300}",
                    role: if msg.starts_with(MSG_ACTION_ERR) { "alert" } else { "status" },
                    "{msg}"
                    if busy() { " ···" }
                }
            }

            // 1. 统计区
            UsersStatsSection { stats }

            // 2. 筛选区
            UsersFilterSection { filter_groups, statuses, roles, search, group_idx, status_idx, role_idx,
                on_refresh: move |_| reload.set(reload() + 1),
                on_new: move |_| form.set(Form::New) }

            // 3. 用户卡片网格
            UsersListSection { filtered, loading, err,
                on_retry: move |_| reload.set(reload() + 1),
                on_edit: open_edit,
                on_topup: move |key: String| topup.set(Some(key)),
                on_toggle: move |(k, a, v): (String, String, Option<String>)| manage_toggle(k, a, v) }
        }

        // 新建 / 编辑弹窗(同款表单;字段信号已按 R2 沉入 UserForm,预填走 user prop)
        if form() != Form::Closed {
            UserForm {
                editing,
                user: edit_user,
                on_cancel: move |_| form.set(Form::Closed),
                // 编辑态:按 tab 回写单字段(set_role / set_groups)
                on_submit: move |(action, value): (String, Option<String>)| {
                    if let Some(k) = edit_key.clone() {
                        manage_form(k, action, value);
                    }
                    form.set(Form::Closed);
                },
                // 新建态:一次性 POST /api/user/users 建号(R3: 只调提取函数)
                on_create: move |req: api::CreateUserRequest| {
                    create_user(notice, reload, busy, req);
                    form.set(Form::Closed);
                },
            }
        }

        // 充值弹窗
        if let Some(key) = topup() {
            TopUpForm {
                user_key: key,
                current_quota: topup_quota,
                on_cancel: move |_| topup.set(None),
                on_submit: move |(k, amount): (String, i64)| {
                    manage_topup(k, "adjust_quota".to_string(), Some(amount.to_string()));
                    topup.set(None);
                },
            }
        }
    }
}
