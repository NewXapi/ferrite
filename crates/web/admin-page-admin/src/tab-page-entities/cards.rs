use dioxus::prelude::*;
use crate::drawer_write::{DrawerNotice, DrawerNoticeBar, create_group_write, delete_group, find_group_by_name, update_group_display};
use crate::tab_page_network::bump_topo_refresh;
use crate::state::EntityStore;
use super::shared::*;
use ui::dialog::Dialog;
// ============ 卡片 1：分组 ============

#[component]
pub fn GroupsCard(open: bool, on_toggle: EventHandler<MouseEvent>) -> Element {
    let store = use_context::<EntityStore>();
    let mut groups = store.groups;
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
        "分组名（锁读，后端无改名路径）"
    } else {
        "分组名"
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
                                        "分组「{cn}」不存在于后端（可能已被删除）"
                                    )));
                                    return;
                                }
                                Err(e) => {
                                    ns.set(DrawerNotice::Err(format!("保存失败：{e}")));
                                    return;
                                }
                            };
                            match update_group_display(&g, &d).await {
                                Ok(_) => {
                                    groups.write()[i].display = d;
                                    bump_topo_refresh();
                                }
                                Err(e) => {
                                    ns.set(DrawerNotice::Err(format!("保存失败：{e}")));
                                    return;
                                }
                            }
                            Ok(())
                        }
                        None => Err("分组行已失效，请刷新后重试".into()),
                    }
                }
            };
            match res {
                Ok(()) => ns.set(DrawerNotice::Ok),
                Err(e) => ns.set(DrawerNotice::Err(format!("保存失败：{e}"))),
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
                    Err(e) => ns.set(DrawerNotice::Err(format!("删除失败：{e}"))),
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
                    Err(e) => ns.set(DrawerNotice::Err(format!("删除失败：{e}"))),
                },
                // 后端已无此名 → 本地行是幻影（创建请求没落地/已被删），只清本地
                Ok(None) => {
                    groups.write().remove(i);
                    ns.set(DrawerNotice::Ok);
                }
                Err(e) => ns.set(DrawerNotice::Err(format!("删除失败：{e}"))),
            }
        });
    };

    rsx! {
        DrawerNoticeBar { notice, on_clear: move |_| notice.set(DrawerNotice::Idle) }
        CardPanel {
            section_index: 0,
            title: "分组",
            hint: "对模型别名分组；分组本身只有名字",
            count: groups.read().len(),
            open: open,
            on_toggle: on_toggle,

            div { class: "flex flex-wrap items-end gap-2",
                InputCell { label: name_label, value: name, placeholder: "vip", grow: true }
                InputCell { label: FIELD_DISPLAY, value: display, placeholder: "默认分组（可选）", grow: true }
                InputCell { label: LBL_MULTIPLIER, value: mult, placeholder: "1.0" }
                button {
                    class: "rounded-md border border-zinc-100 bg-zinc-100 px-3 py-1.5 text-xs font-medium text-zinc-900 hover:bg-zinc-300",
                    disabled: saving(),
                    onclick: commit,
                    if editing().is_some() { {BTN_UPDATE} } else { {BTN_NEW} }
                }
                if editing().is_some() {
                    button {
                        class: "rounded-md border border-zinc-800 px-3 py-1.5 text-xs text-zinc-400 hover:border-zinc-600 hover:text-zinc-200",
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
                    EmptyHint { text: "还没有分组" }
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
                        title: "删除分组".to_string(),
                        open: true,
                        on_confirm: confirm_delete,
                        on_cancel: move |_| confirming.set(None),
                        div { class: "text-xs text-zinc-400", "确认删除分组「{cname}」？该操作直接生效于后端，不可撤销。" }
                    }
                }
            } else {
                rsx! { Fragment {} }
            }
        }
    }
}

// ============ 卡片 2：模型别名 ============

#[component]
pub fn AliasesCard(open: bool, on_toggle: EventHandler<MouseEvent>) -> Element {
    let store = use_context::<EntityStore>();
    let mut aliases = store.aliases;
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
            title: "模型别名",
            hint: "对外暴露给用户的模型名；卡牌样式后续再做",
            count: aliases.read().len(),
            open: open,
            on_toggle: on_toggle,

            div { class: "flex flex-wrap items-end gap-2",
                InputCell { label: "别名", value: name, placeholder: "gpt-4o", grow: true }
                InputCell { label: FIELD_DISPLAY, value: display, placeholder: "GPT-4o（可选）", grow: true }
                InputCell { label: "输入价 ¥/1k", value: input_rate, placeholder: "0.0175" }
                InputCell { label: "输出价 ¥/1k", value: output_rate, placeholder: "0.07" }
                InputCell { label: LBL_MULTIPLIER, value: mult, placeholder: "1.0" }
                button {
                    class: "rounded-md border border-zinc-100 bg-zinc-100 px-3 py-1.5 text-xs font-medium text-zinc-900 hover:bg-zinc-300",
                    onclick: commit,
                    if editing().is_some() { {BTN_UPDATE} } else { {BTN_NEW} }
                }
                if editing().is_some() {
                    button {
                        class: "rounded-md border border-zinc-800 px-3 py-1.5 text-xs text-zinc-400 hover:border-zinc-600 hover:text-zinc-200",
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
                    EmptyHint { text: "还没有模型别名" }
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

