//! 实体设置页的两张纯本地卡片:分组卡(接后端 CRUD)与模型别名卡(演示态本地行)。
//!
//! 负责:分组的新增/改名展示/删除(走 `drawer_write` 真实端点)与别名卡的
//! 本地行增删改(后端 models 域无对应列,仅 UI 层演示)。
//!
//! 不负责:渠道卡(`channels.rs`)、三卡展开态(`page.rs`)、网络层实现
//! (`crate::drawer_write`)。

use super::shared::*;
use crate::drawer_write::{
    DrawerNotice, DrawerNoticeBar, create_group_write, delete_group, find_group_by_name,
    update_group_display,
};
use crate::state::EntityStore;
use crate::tab_page_network::bump_topo_refresh;
use dioxus::prelude::*;
use ui::dialog::Dialog;

// ============ 卡片 1：分组 ============

/// 分组卡：分组的新增 / 选中编辑 / 删除确认。
///
/// 【是什么】实体设置页的第一张卡:录入行(分组名/展示名/倍率 + 提交按钮)+ 下方分组胶囊区。
///
/// 【做什么】负责分组的新建(POST `/api/group`)与展示名更新(`update_group_display`)、
/// 删除确认弹窗,并把结果折算成顶部 `DrawerNoticeBar` 的提示。不负责渠道与别名
/// (`cards.rs` 的兄弟卡 / `channels.rs`)、不负责卡片的折叠态(`page.rs` 持有)。
/// 分组名在后端无改名路径,编辑态下只读。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 在录入行输入 → 就地写 `name` / `display` / `mult` 三 Signal(无网络)。
/// - 点「新增/更新」→ `commit`:新建走 `create_group_write`,编辑先
///   `find_group_by_name` 再 `update_group_display`;成功后写本地 `groups` 行、
///   `bump_topo_refresh()` 刷拓扑画布,并置 `notice`。
/// - 点胶囊 → 回填三 Signal 并置 `editing = Some(i)`(无网络);点胶囊 ✕ → 打开确认弹窗。
/// - 确认删除 → `delete_group`;后端已无此名时只清本地行并按成功提示。
/// 写操作期间 `saving` 为真会禁用提交按钮,避免双击重复提交。
///
/// 【样式】卡片外壳与头部由 `CardPanel` 提供;录入行 `flex flex-wrap items-end gap-2`;
/// 提交按钮 `rounded-md border-zinc-100 bg-zinc-100 ... hover:bg-zinc-300`,取消按钮
/// `border-zinc-800 text-zinc-400 hover:border-zinc-600`;胶囊区在 `NodeArea` 里
/// `flex flex-wrap gap-2`;确认弹窗用 `ui::dialog::Dialog`。
///
/// 【子组件组成】`DrawerNoticeBar`(写操作提示条)、`CardPanel`(卡外壳)、
/// `InputCell`(三个录入格)、`NodeArea`(胶囊槽)、`EmptyHint`(空态)、
/// `EntityChip`(单个分组胶囊)、`Dialog`(删除确认,条件渲染)。
///
/// 【数据流】
/// - 对内(入):`open`(卡片展开态,页面持有)、`on_toggle`(切展开);
///   分组数据来自 `use_context::<EntityStore>()` 的 `groups`(本地 store 行,
///   仅作拓扑启动布局兜底),而非 prop。
/// - 对外(出):写本地 `groups` 行、置 `notice`、调 `bump_topo_refresh()` 通知画布重拉;
///   `on_toggle` 冒泡给页面翻转 `open` 数组。
#[component]
pub fn GroupsCard(open: bool, on_toggle: EventHandler<MouseEvent>) -> Element {
    let store = use_context::<EntityStore>();
    let mut groups = store.groups;
    // —— 录入行状态:仅本卡使用,不跨组件,故就地持有 ——
    let mut name = use_signal(String::new);
    let mut display = use_signal(String::new);
    let mut mult = use_signal(String::new);
    let mut editing = use_signal(|| None::<usize>);
    // 写操作反馈（顶部 DrawerNoticeBar）与删除确认弹窗
    let mut notice = use_signal(|| DrawerNotice::Idle);
    let mut confirming = use_signal(|| None::<usize>);
    let mut saving = use_signal(|| false);
    // 编辑态下分组名锁读（后端 UpdateGroupRequest 无 name 列），标签如实标注
    let name_label: &'static str = if editing.peek().is_some() {
        FIELD_GROUP_NAME_LOCKED
    } else {
        FIELD_GROUP_NAME
    };

    let commit = move |_| {
        if *saving.peek() {
            return; // 上一次写还在途，防双击重复提交
        }
        let n = name.peek().trim().to_string();
        if n.is_empty() {
            return;
        }
        saving.set(true);
        let d = display.peek().trim().to_string();
        let m = parse_mult(&mult.peek());
        let mode = *editing.peek();
        let mut ns = notice;
        spawn(async move {
            // 写结果折算成 DrawerNotice（成功 / 失败摘要 / 进行中）
            let res: Result<(), String> = match mode {
                // 新建：真实 POST /api/group
                None => match create_group_write(&n, &d, m).await {
                    Ok(g) => {
                        groups.write().push(crate::state::GroupRow {
                            key: g.key,
                            name: n,
                            display: d,
                            multiplier: m,
                        });
                        bump_topo_refresh();
                        Ok(())
                    }
                    Err(e) => Err(e),
                },
                // 编辑：分组名后端无更新路径（锁读），只有展示名/备注可写
                Some(i) => {
                    let current = groups.read().get(i).map(|r| r.name.clone());
                    match current {
                        Some(cn) => {
                            let g = match find_group_by_name(&cn).await {
                                Ok(Some(g)) => g,
                                Ok(None) => {
                                    ns.set(DrawerNotice::Err(format!(
                                        "{MSG_GROUP_MISSING_PREFIX}{cn}{MSG_GROUP_MISSING_SUFFIX}"
                                    )));
                                    return;
                                }
                                Err(e) => {
                                    ns.set(DrawerNotice::Err(format!(
                                        "{MSG_SAVE_FAILED_PREFIX}{e}"
                                    )));
                                    return;
                                }
                            };
                            match update_group_display(&g, &d).await {
                                Ok(_) => {
                                    groups.write()[i].display = d;
                                    bump_topo_refresh();
                                }
                                Err(e) => {
                                    ns.set(DrawerNotice::Err(format!(
                                        "{MSG_SAVE_FAILED_PREFIX}{e}"
                                    )));
                                    return;
                                }
                            }
                            Ok(())
                        }
                        None => Err(MSG_ROW_STALE.into()),
                    }
                }
            };
            match res {
                Ok(()) => ns.set(DrawerNotice::Ok),
                Err(e) => ns.set(DrawerNotice::Err(format!("{MSG_SAVE_FAILED_PREFIX}{e}"))),
            }
            saving.set(false);
        });
        name.set(String::new());
        display.set(String::new());
        mult.set(String::new());
        editing.set(None);
    };

    let mut request_delete = move |i: usize| {
        confirming.set(Some(i));
    };

    let confirm_delete = move |_| {
        let Some(i) = *confirming.peek() else {
            return;
        };
        confirming.set(None);
        let row = groups.read().get(i).cloned();
        let Some(r) = row else {
            return;
        };
        let local_key = r.key.clone();
        let mut ns = notice;
        spawn(async move {
            // 行 key 优先；seed 演示行 key 为空才按名称找
            if !local_key.is_empty() {
                match delete_group(&local_key).await {
                    Ok(()) => {
                        groups.write().remove(i);
                        bump_topo_refresh();
                        ns.set(DrawerNotice::Ok);
                    }
                    Err(e) => ns.set(DrawerNotice::Err(format!("{MSG_DELETE_FAILED_PREFIX}{e}"))),
                }
                return;
            }
            match find_group_by_name(&r.name).await {
                Ok(Some(g)) => match delete_group(&g.key).await {
                    Ok(()) => {
                        groups.write().remove(i);
                        bump_topo_refresh();
                        ns.set(DrawerNotice::Ok);
                    }
                    Err(e) => ns.set(DrawerNotice::Err(format!("{MSG_DELETE_FAILED_PREFIX}{e}"))),
                },
                // 后端已无此名 → 本地行是幻影（创建请求没落地/已被删），只清本地
                Ok(None) => {
                    groups.write().remove(i);
                    ns.set(DrawerNotice::Ok);
                }
                Err(e) => ns.set(DrawerNotice::Err(format!("{MSG_DELETE_FAILED_PREFIX}{e}"))),
            }
        });
    };

    rsx! {
        DrawerNoticeBar { notice, on_clear: move |_| notice.set(DrawerNotice::Idle) }
        CardPanel {
            section_index: 0,
            title: SEC_CARD_GROUPS,
            hint: SEC_CARD_GROUPS_HINT,
            count: groups.read().len(),
            open: open,
            on_toggle: on_toggle,

            div { class: "flex flex-wrap items-end gap-2",
                InputCell { label: name_label, value: name, placeholder: MSG_PH_GROUP_NAME, grow: true }
                InputCell { label: FIELD_DISPLAY, value: display, placeholder: MSG_PH_GROUP_DISPLAY, grow: true }
                InputCell { label: LBL_MULTIPLIER, value: mult, placeholder: "1.0" }
                button {
                    class: "rounded-md border {ui::T_border_zinc_100} {ui::T_bg_zinc_100} px-3 py-1.5 {ui::T_text_xs} {ui::T_font_medium} {ui::T_text_zinc_900} hover:{ui::T_bg_zinc_300}",
                    disabled: saving(),
                    onclick: commit,
                    if editing().is_some() { {BTN_UPDATE} } else { {BTN_NEW} }
                }
                if editing().is_some() {
                    button {
                        class: "rounded-md border {ui::T_border_zinc_800} px-3 py-1.5 {ui::T_text_xs} {ui::T_text_zinc_400} hover:{ui::T_border_zinc_600} hover:{ui::T_text_zinc_200}",
                        onclick: move |_| {
                            editing.set(None);
                            name.set(String::new());
                            display.set(String::new());
                            mult.set(String::new());
                        },
                        {BTN_CANCEL}
                    }
                }
            }

            NodeArea {
                if groups.read().is_empty() {
                    EmptyHint { text: MSG_EMPTY_GROUPS }
                } else {
                    div { class: "flex flex-wrap gap-2",
                        for (i, r) in groups.read().iter().enumerate() {
                            EntityChip {
                                label: r.name.clone(),
                                sub: r.display.clone(),
                                active: editing() == Some(i),
                                on_pick: move |_| {
                                    let r = groups.read()[i].clone();
                                    name.set(r.name);
                                    display.set(r.display);
                                    mult.set(format!("{}", r.multiplier));
                                    editing.set(Some(i));
                                },
                                on_remove: move |_| request_delete(i),
                            }
                        }
                    }
                }
            }
        }

        // 删除确认弹窗（ui_components::dialog::Dialog）
        {
            let confirming_now = *confirming.peek();
            if let Some(ci) = confirming_now {
                let cname = groups.read().get(ci).map(|r| r.name.clone()).unwrap_or_default();
                rsx! {
                    Dialog {
                        title: TTL_DELETE_GROUP.to_string(),
                        open: true,
                        on_confirm: confirm_delete,
                        on_cancel: move |_| confirming.set(None),
                        div { class: "{ui::T_text_xs} {ui::T_text_zinc_400}", "{MSG_CONFIRM_DELETE_GROUP_PREFIX}{cname}{MSG_CONFIRM_DELETE_SUFFIX}" }
                    }
                }
            } else {
                rsx! { Fragment {} }
            }
        }
    }
}

// ============ 卡片 2：模型别名 ============

/// 模型别名卡：演示态本地行的录入 / 选中编辑 / 删除。
///
/// 【是什么】实体设置页的第二张卡:录入行(别名/展示名/输入价/输出价/倍率)+ 下方别名胶囊区。
///
/// 【做什么】只维护 `EntityStore.aliases` 的本地行(增删改),**不接后端**——后端
/// models 域只有 `name` 列(display/价格/倍率无对应列,PUT 会静默丢弃),create 还
/// 必填 owner 与 api_key(表单无来源)。刷新即丢,不伪装成功,也不留 todo! 占位
/// (那会在用户可触发的提交路径上 panic)。不负责分组卡与渠道卡,不负责真实定价。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 在录入行输入 → 就地写五个 Signal(无网络)。
/// - 点「新增/更新」→ `commit` 就地改 `aliases` 本地行(编辑按下标写回、新增 push),
///   随即清空录入行;不发起任何请求。
/// - 点胶囊 → 回填五个 Signal 并置 `editing = Some(i)`;点胶囊 ✕ → 直接删本地行并
///   在命中当前编辑行时退出编辑态(无确认弹窗,因为不触后端)。
///
/// 【样式】卡片外壳与头部由 `CardPanel` 提供;录入行 `flex flex-wrap items-end gap-2`;
/// 提交/取消按钮样式与分组卡一致;胶囊区在 `NodeArea` 里 `flex flex-wrap gap-2`。
///
/// 【子组件组成】`CardPanel`(卡外壳)、`InputCell`(五个录入格)、`NodeArea`(胶囊槽)、
/// `EmptyHint`(空态)、`EntityChip`(单个别名胶囊)。
///
/// 【数据流】
/// - 对内(入):`open`(卡片展开态,页面持有)、`on_toggle`(切展开);
///   别名数据来自 `use_context::<EntityStore>()` 的 `aliases`。
/// - 对外(出):写本地 `aliases` 行(纯 UI 层,不发网络);`on_toggle` 冒泡给页面。
#[component]
pub fn AliasesCard(open: bool, on_toggle: EventHandler<MouseEvent>) -> Element {
    let store = use_context::<EntityStore>();
    let mut aliases = store.aliases;
    // —— 录入行状态:仅本卡使用,不跨组件,故就地持有 ——
    let mut name = use_signal(String::new);
    let mut display = use_signal(String::new);
    let mut input_rate = use_signal(String::new);
    let mut output_rate = use_signal(String::new);
    let mut mult = use_signal(String::new);
    let mut editing = use_signal(|| None::<usize>);
    // 别名卡维持演示态本地行（B1 范围外，不接后端）：后端 models 域只有
    // name 列（display/价格/倍率无对应列，PUT 会静默丢弃），create 还必填
    // owner 与 api_key（表单无来源）。接真实写路径需先把表单收成仅别名一列，
    // 属独立改造；这里保留 store 本地行（刷新即丢，不伪装成功），
    // 绝不用 todo! 占位——那会在用户可触发的提交路径上直接 panic。
    let commit = move |_| {
        let n = name.peek().trim().to_string();
        if n.is_empty() {
            return;
        }
        let d = display.peek().trim().to_string();
        match *editing.peek() {
            Some(i) => {
                if let Some(r) = aliases.write().get_mut(i) {
                    r.alias = n;
                    r.display = d;
                    r.input_per_1k = parse_nonneg(&input_rate.peek());
                    r.output_per_1k = parse_nonneg(&output_rate.peek());
                    r.multiplier = parse_mult(&mult.peek());
                }
            }
            None => aliases.write().push(crate::state::AliasRow {
                alias: n,
                display: d,
                input_per_1k: parse_nonneg(&input_rate.peek()),
                output_per_1k: parse_nonneg(&output_rate.peek()),
                multiplier: parse_mult(&mult.peek()),
            }),
        }
        name.set(String::new());
        display.set(String::new());
        input_rate.set(String::new());
        output_rate.set(String::new());
        mult.set(String::new());
        editing.set(None);
    };

    rsx! {
        CardPanel {
            section_index: 1,
            title: SEC_CARD_ALIASES,
            hint: SEC_CARD_ALIASES_HINT,
            count: aliases.read().len(),
            open: open,
            on_toggle: on_toggle,

            div { class: "flex flex-wrap items-end gap-2",
                InputCell { label: FIELD_ALIAS, value: name, placeholder: "gpt-4o", grow: true }
                InputCell { label: FIELD_DISPLAY, value: display, placeholder: MSG_PH_ALIAS_DISPLAY, grow: true }
                InputCell { label: FIELD_INPUT_PRICE, value: input_rate, placeholder: "0.0175" }
                InputCell { label: FIELD_OUTPUT_PRICE, value: output_rate, placeholder: "0.07" }
                InputCell { label: LBL_MULTIPLIER, value: mult, placeholder: "1.0" }
                button {
                    class: "rounded-md border {ui::T_border_zinc_100} {ui::T_bg_zinc_100} px-3 py-1.5 {ui::T_text_xs} {ui::T_font_medium} {ui::T_text_zinc_900} hover:{ui::T_bg_zinc_300}",
                    onclick: commit,
                    if editing().is_some() { {BTN_UPDATE} } else { {BTN_NEW} }
                }
                if editing().is_some() {
                    button {
                        class: "rounded-md border {ui::T_border_zinc_800} px-3 py-1.5 {ui::T_text_xs} {ui::T_text_zinc_400} hover:{ui::T_border_zinc_600} hover:{ui::T_text_zinc_200}",
                        onclick: move |_| {
                            editing.set(None);
                            name.set(String::new());
                            display.set(String::new());
                            input_rate.set(String::new());
                            output_rate.set(String::new());
                            mult.set(String::new());
                        },
                        {BTN_CANCEL}
                    }
                }
            }

            NodeArea {
                if aliases.read().is_empty() {
                    EmptyHint { text: MSG_EMPTY_ALIASES }
                } else {
                    div { class: "flex flex-wrap gap-2",
                        for (i, r) in aliases.read().iter().enumerate() {
                            EntityChip {
                                label: r.alias.clone(),
                                sub: r.display.clone(),
                                active: editing() == Some(i),
                                on_pick: move |_| {
                                    let r = aliases.read()[i].clone();
                                    name.set(r.alias);
                                    display.set(r.display);
                                    input_rate.set(format!("{}", r.input_per_1k));
                                    output_rate.set(format!("{}", r.output_per_1k));
                                    mult.set(format!("{}", r.multiplier));
                                    editing.set(Some(i));
                                },
                                on_remove: move |_| {
                                    // 演示态本地行：只删本地，不触后端（见 commit 处注释）
                                    aliases.write().remove(i);
                                    if editing() == Some(i) { editing.set(None); }
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}
