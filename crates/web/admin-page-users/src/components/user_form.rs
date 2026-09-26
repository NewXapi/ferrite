//! 新建 / 编辑用户弹窗(同款表单)。
//!
//! 弹窗内是三个**真实**页签:每组只渲染自己的字段,切 tab 真正切内容。
//! 旧版四个 tab(基本/额度/备注/绑定)里「基本/额度/备注」共用同一组字段,
//! 切换只是滚动到不同输入框——视觉上是 tab,语义上不是。
//!
//! 保存行为按态分流:
//! - 编辑态:按 tab 回写单字段(`set_role` / `set_groups`),绑定页只读无写路径
//! - 新建态:一次性 `POST /api/user/users` 建号

use dioxus::prelude::*;

use contract::api::admin::AdminUserDto;

use crate::api;
use crate::format::fmt_cny;

use super::chip::{Chip, ChipSize};
use super::group_chips::GroupChips;
use super::modal::{MODAL_INPUT, Modal};
use super::role_chips::RoleChips;
use crate::shared::{
    BTN_CANCEL, BTN_CREATE, BTN_SAVE, FIELD_GROUP_HINT, FIELD_GROUPS, FIELD_INIT_PASSWORD,
    FIELD_NOTE, FIELD_NOTE_HINT, FIELD_PASSWORD_HINT, FIELD_ROLE, FIELD_USERNAME,
    FIELD_USERNAME_HINT, LBL_EMAIL, LBL_QUOTA, MSG_BINDING_READONLY, MSG_QUOTA_HINT, TAB_BASIC,
    TAB_BINDING, TAB_GROUP, TTL_EDIT_USER, TTL_NEW_USER,
};

/// 编辑弹窗内的真实页签。
///
/// - `Basic`:用户名 / 邮箱 / 角色 / 剩余额度(可编辑生效)
/// - `Group`:生效分组(多选) / 管理员备注
/// - `Binding`:第三方绑定(只读,后端暂无此列)
#[derive(Clone, Copy, PartialEq)]
pub enum FormTab {
    Basic,
    Group,
    Binding,
}

/// 页签顺序与文案;`data-testid` 直接取这里的 label。
pub const TAB_LABELS: [(FormTab, &str); 3] = [
    (FormTab::Basic, TAB_BASIC),
    (FormTab::Group, TAB_GROUP),
    (FormTab::Binding, TAB_BINDING),
];

/// 新建 / 编辑用户弹窗(三个真实页签)。
///
/// 【是什么】用户的新建与编辑浮层:Basic(用户名/邮箱/角色/额度)、Group(生效分组/
/// 备注)、Binding(第三方绑定只读)三个真实页签,每组只渲染自己的字段。
///
/// 【做什么】渲染并收集表单字段、按态分流提交(编辑: 按 tab 回写单字段
/// `set_role` / `set_groups`;新建: 一次性 POST 建号)。不负责网络请求本身
/// (经 EventHandler 抛回页面执行)。
///
/// 【交互逻辑】输入/切页签 → 组件内信号;保存 → 编辑态 `on_submit((action, value))`
/// / 新建态 `on_create(CreateUserRequest)`;取消 → `on_cancel`。
///
/// 【样式】`Modal` 外壳 + 圆角页签按钮(选中反色)+ `MODAL_INPUT` 输入框。
///
/// 【子组件组成】`Modal`、`Chip`(页签行)、`GroupChips`、`RoleChips`。
///
/// 【数据流】
/// - 对内(入):`editing`(态分流)、`user`(预填源——编辑态的用户对象,挂载时
///   注水字段信号;None = 新建默认值)。
/// - 对外(出):`on_cancel` / `on_submit((action, value))` / `on_create(req)`。
#[component]
pub fn UserForm(
    editing: bool,
    user: Option<AdminUserDto>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<(String, Option<String>)>,
    on_create: EventHandler<api::CreateUserRequest>,
) -> Element {
    // —— 表单字段信号(R2 下沉):仅本弹窗读写,页面不再持有 ——
    let mut username = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut quota = use_signal(|| "5000000".to_string());
    // 生效分组(多值,对齐 `auth_users.groups`;后端 `set_groups` 整体替换)
    let mut group = use_signal(|| vec!["default".to_string()]);
    let mut remark = use_signal(String::new);
    let mut role = use_signal(|| 10u16);

    // 预填(use_hook 首次渲染期执行,打开即见值,无闪帧):
    // 编辑态按 user 注水;新建态回默认——与旧 open_new/open_edit 逐字段等价
    // (新建 role=1;编辑 role/额度/分组取当前用户,remark 恒空)。
    use_hook(|| match &user {
        Some(u) => {
            username.set(u.username.clone());
            email.set(u.email.clone());
            password.set(String::new());
            quota.set(u.quota.to_string());
            group.set(u.groups.clone());
            remark.set(String::new());
            role.set(u.role);
        }
        None => {
            username.set(String::new());
            email.set(String::new());
            password.set(String::new());
            quota.set("5000000".to_string());
            group.set(vec!["default".to_string()]);
            remark.set(String::new());
            role.set(1);
        }
    });
    let title = if editing { TTL_EDIT_USER } else { TTL_NEW_USER };
    let submit_label = if editing { BTN_SAVE } else { BTN_CREATE };
    // 额度输入的元换算提示:内部单位 → 人民币,不暴露 quota 字样
    let quota_hint = quota()
        .trim()
        .parse::<i64>()
        .map(fmt_cny)
        .unwrap_or_else(|_| "—".to_string());
    let mut tab = use_signal(|| FormTab::Basic);

    // 绑定页邮箱展示:避免对临时 String 取引用导致其被提前释放。
    let email_str = email();
    let email_bound: &str = if email_str.is_empty() {
        "-"
    } else {
        &email_str
    };

    // 保存:编辑态按 tab 回写单字段,新建态走 on_create 一次性建号。
    let do_submit = move || {
        if editing {
            match tab() {
                FormTab::Basic => {
                    on_submit.call(("set_role".to_string(), Some(role().to_string())));
                }
                // 多分组整体替换:后端 set_groups 收逗号/空白分隔列表
                FormTab::Group => {
                    on_submit.call(("set_groups".to_string(), Some(group().join(","))));
                }
                // 绑定页只读,无写路径(按钮也不渲染)
                FormTab::Binding => {}
            }
        } else {
            on_create.call(api::CreateUserRequest {
                username: username(),
                password: password(),
                email: if email().trim().is_empty() {
                    None
                } else {
                    Some(email())
                },
                role: role(),
                quota: quota().trim().parse().unwrap_or(0),
                groups: group(),
            });
        }
    };

    rsx! {
        Modal { title: title.to_string(), on_close: move |_| on_cancel.call(()),
            // 页签行:弹窗顶部,手机端自动折行
            div { class: "mb-4 flex flex-wrap gap-1.5",
                for (t, label) in TAB_LABELS {
                    Chip {
                        label: label.to_string(),
                        selected: tab() == t,
                        testid: format!("user-form-tab-{label}"),
                        size: ChipSize::Md,
                        on_press: move |_| tab.set(t),
                    }
                }
            }

            // —— Tab 1:基本信息(用户名/邮箱/初始密码/角色/额度) ——
            if tab() == FormTab::Basic {
                div { class: "space-y-4",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_USERNAME}" }
                        input {
                            class: MODAL_INPUT,
                            placeholder: FIELD_USERNAME_HINT,
                            value: "{username}",
                            oninput: move |e| username.set(e.value()),
                        }
                    }
                    // 初始密码仅新建时填写;编辑态改密走独立 reset_password 动作
                    if !editing {
                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_INIT_PASSWORD}" }
                            input {
                                class: MODAL_INPUT,
                                r#type: "password",
                                placeholder: FIELD_PASSWORD_HINT,
                                value: "{password}",
                                oninput: move |e| password.set(e.value()),
                            }
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "{LBL_EMAIL}" }
                        input {
                            class: MODAL_INPUT,
                            r#type: "email",
                            placeholder: "user@example.com",
                            value: "{email}",
                            oninput: move |e| email.set(e.value()),
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_ROLE}" }
                        RoleChips { role, on_change: move |v: u16| role.set(v) }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "{LBL_QUOTA}" }
                        input {
                            class: "{MODAL_INPUT} font-mono",
                            r#type: "text",
                            value: "{quota}",
                            oninput: move |e| quota.set(e.value()),
                        }
                        p { class: "mt-1 {ui::TYPE_DESC}", "{MSG_QUOTA_HINT} {quota_hint}" }
                    }
                }
            }

            // —— Tab 2:分组与备注 ——
            if tab() == FormTab::Group {
                div { class: "space-y-4",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_GROUPS}" }
                        GroupChips { group, on_change: move |v: Vec<String>| group.set(v) }
                        p { class: "mt-1 {ui::TYPE_DESC}", "{FIELD_GROUP_HINT}" }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_NOTE}" }
                        textarea {
                            class: "{MODAL_INPUT} h-24 resize-none",
                            placeholder: FIELD_NOTE_HINT,
                            value: "{remark}",
                            oninput: move |e| remark.set(e.value()),
                        }
                    }
                }
            }

            // —— Tab 3:绑定(只读) ——
            if tab() == FormTab::Binding {
                div { class: "rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 text-xs",
                    p { class: "mb-2 text-[11px] text-zinc-500", "{MSG_BINDING_READONLY}" }
                    div { class: "space-y-1.5",
                        for (label, value) in [("GitHub", "-"), ("Discord", "-"), ("OIDC", "-"), ("WeChat", "-"), ("Telegram", "-"), (LBL_EMAIL, email_bound)] {
                            div { class: "flex justify-between gap-2",
                                span { class: "text-zinc-400", "{label}" }
                                span { class: "font-medium text-zinc-200", "{value}" }
                            }
                        }
                    }
                }
            }

            div { class: "mt-6 flex gap-3",
                button {
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                    onclick: move |_| on_cancel.call(()),
                    {BTN_CANCEL}
                }
                if tab() != FormTab::Binding {
                    button {
                        class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200",
                        // 每个 tab 只回写自己负责的字段(角色 / 分组)。
                        onclick: move |_| do_submit(),
                        "{submit_label}"
                    }
                }
            }
        }
    }
}
