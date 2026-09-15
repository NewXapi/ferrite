//! 实体设置页：分组 / 模型别名 / 渠道 三张可折叠卡片，各占一行。
//! 每张卡片上半是录入行，下半是该实体在拓扑里对应的节点内容。
//!
//! 数据在 `crate::state::EntityStore` 中（拓扑启动布局兜底快照）；
//! 写路径一律走 `crate::drawer_write` 的真实端点（分组/渠道 CRUD），
//! 成功后由调用方重拉 `network::load_network_data` 刷新画布——
//! #183 起画布不再由 store 行驱动，本地 store 行仅作启动布局兜底。

use dioxus::prelude::*;
use ui::Dialog;

use crate::drawer_write::{
    DrawerNotice, DrawerNoticeBar, create_channel_import, create_group_write, delete_channel,
    delete_group, find_channel_by_name, find_group_by_name, set_channel_status, update_channel,
    update_group_display,
};
use crate::network::bump_topo_refresh;
use crate::state::EntityStore;

#[component]
pub fn EntitiesPanel() -> Element {
    let mut open = use_signal(|| [true, true, true]);

    rsx! {
        // 滚动由外层容器（拓扑抽屉）负责，这里别自带 overflow，
        // 否则锚点/滚动事件会对不上元素。
        div { class: "flex flex-col gap-3",
            GroupsCard {
                open: open()[0],
                on_toggle: move |_| { let mut o = open(); o[0] = !o[0]; open.set(o); },
            }
            AliasesCard {
                open: open()[1],
                on_toggle: move |_| { let mut o = open(); o[1] = !o[1]; open.set(o); },
            }
            ChannelsCard {
                open: open()[2],
                on_toggle: move |_| { let mut o = open(); o[2] = !o[2]; open.set(o); },
            }
        }
    }
}

// —— 跨卡片共享文案 (GroupsCard / AliasesCard 同用) ——
const FIELD_DISPLAY: &str = "展示名";
const LBL_MULTIPLIER: &str = "倍率";
const BTN_NEW: &str = "新增";
const BTN_UPDATE: &str = "更新";
const BTN_CANCEL: &str = "取消";

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

/// 非负单价解析:空/非法回退 0,负数归零
fn parse_nonneg(s: &str) -> f64 {
    s.trim().parse::<f64>().unwrap_or(0.0).max(0.0)
}

/// 倍率解析:空/非法回退 1.0,负数归零
fn parse_mult(s: &str) -> f64 {
    s.trim().parse::<f64>().unwrap_or(1.0).max(0.0)
}

// ============ 卡片 3：渠道 ============

#[component]
pub fn ChannelsCard(open: bool, on_toggle: EventHandler<MouseEvent>) -> Element {
    let store = use_context::<EntityStore>();
    let mut channels = store.channels;
    let mut current = use_signal(|| 0usize);
    let mut name = use_signal(String::new);
    let mut ctype = use_signal(String::new);
    let mut url = use_signal(String::new);
    let mut keys = use_signal(String::new);
    let mut status = use_signal(|| 0u8);
    let mut is_new = use_signal(|| false);
    let mut notice = use_signal(|| DrawerNotice::Idle);
    let mut confirming = use_signal(|| None::<usize>);
    let mut saving = use_signal(|| false);

    let idx = current();
    let row = channels.read().get(idx).cloned();

    // 载入行到编辑区（新建渠道 = 空行 + is_new）
    let mut load_row = move |i: usize| {
        let binding = channels.read();
        let Some(r) = binding.get(i) else {
            return;
        };
        name.set(r.name.clone());
        ctype.set(r.ctype.clone());
        url.set(r.url.clone());
        keys.set(String::new()); // 掩码值绝不回传：编辑区 keys 固定留空 = 不覆盖
        status.set(r.status);
        is_new.set(false);
    };

    let mut load_new = move |_| {
        name.set("新渠道".into());
        ctype.set("openai".into());
        url.set(String::new());
        keys.set(String::new());
        status.set(1);
        is_new.set(true);
    };

    // 保存：新建走 create_channel_import；编辑走 update_channel（最小 diff，
    // keys 仅当用户本次重输时携带；未重输 = 请求体缺席 keys 字段 = 保留现值）。
    // 成功后 bump_topo_refresh() 刷画布。
    let save = move |_| {
        if *saving.peek() {
            return; // 上一次写还在途，防双击重复提交
        }
        let n = name.peek().trim().to_string();
        saving.set(true);
        if n.is_empty() {
            return;
        }
        let ct = ctype.peek().trim().to_string();
        let u = url.peek().trim().to_string();
        let raw_keys = keys.peek().trim().to_string();
        let kvec: Vec<String> = raw_keys
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let st = *status.peek();
        let new = *is_new.peek();
        let i = *current.peek();
        // 新建渠道至少要一个明文 key（后端 validate 必 400，提前拦省一趟）
        if new && kvec.is_empty() {
            notice.set(DrawerNotice::Err("新建渠道至少填写一个 API Key".into()));
            return;
        }
        // 新建 = default 分组；编辑取当前行既有分组
        let grp: Vec<String> = if new {
            vec!["default".to_string()]
        } else {
            channels
                .read()
                .get(i)
                .map(|c| {
                    c.group
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_else(|| vec!["default".to_string()])
        };
        let cur_name = channels
            .read()
            .get(i)
            .map(|c| c.name.clone())
            .unwrap_or_default();
        let (mut ns, mut ch_sig, mut cur_sig, mut new_sig) = (notice, channels, current, is_new);
        spawn(async move {
            let res: Result<(), String> = if new {
                let created = match create_channel_import(&n, &u, &ct, &grp, &kvec).await {
                    Ok(c) => c,
                    Err(e) => {
                        ns.set(DrawerNotice::Err(format!("保存失败：{e}")));
                        return;
                    }
                };
                ch_sig.write().push(crate::state::ChannelRow {
                    name: n.clone(),
                    key: created.key.clone(),
                    ctype: ct.clone(),
                    url: u.clone(),
                    keys: created.keys.map(|v| v.join("\n")).unwrap_or_default(),
                    status: st,
                    group: grp.join(","),
                    latency_ms: None,
                    candidates: vec![],
                    dispatch: crate::network::channel_models(&created.models),
                });
                // 落库成功后切到新行编辑态：否则 is_new 悬着，下一次保存
                // 会再建一个重复渠道。
                let last = ch_sig.read().len().saturating_sub(1);
                cur_sig.set(last);
                new_sig.set(false);
                Ok(())
            } else {
                // 优先用行内服务端 key（hydrate/新建时已灌入，免一趟查询且无
                // 外部改名竞态）；seed 演示行 key 为空才按名称找。Ok(None)
                // （列表成功但无此名）才按导入兜底建渠道；Err（401/网络/5xx）
                // 直接报错——绝不能把一次瞬时故障降级成新建，那会复制出重复渠道。
                let local_key = ch_sig
                    .read()
                    .get(i)
                    .map(|c| c.key.clone())
                    .unwrap_or_default();
                let key = if !local_key.is_empty() {
                    local_key
                } else {
                    match find_channel_by_name(&cur_name).await {
                        Ok(Some(c)) => c.key,
                        Ok(None) => match create_channel_import(&n, &u, &ct, &grp, &kvec).await {
                            Ok(c) => c.key,
                            Err(e) => {
                                ns.set(DrawerNotice::Err(format!("保存失败：{e}")));
                                return;
                            }
                        },
                        Err(e) => {
                            ns.set(DrawerNotice::Err(format!("保存失败：{e}")));
                            return;
                        }
                    }
                };
                // 用户未重输 keys → 传 None（请求体缺席 keys 字段 = 保留现值）
                let new_keys = (!kvec.is_empty()).then(|| kvec.clone());
                if let Err(e) = update_channel(&key, &n, &u, new_keys).await {
                    ns.set(DrawerNotice::Err(format!("保存失败：{e}")));
                    return;
                }
                if let Some(r) = ch_sig.write().get_mut(i) {
                    r.name = n;
                    r.url = u;
                    if !kvec.is_empty() {
                        r.keys = kvec.join("\n");
                    }
                }
                Ok(())
            };
            match res {
                Ok(()) => {
                    ns.set(DrawerNotice::Ok);
                    bump_topo_refresh();
                }
                Err(e) => ns.set(DrawerNotice::Err(format!("保存失败：{e}"))),
            }
            saving.set(false);
        });
    };

    // 启停切换
    let toggle_status = move |_| {
        let i = *current.peek();
        let cur = channels.read().get(i).cloned();
        let Some(r) = cur else { return };
        let target: u8 = if r.status == 1 { 2 } else { 1 };
        let cur_name = r.name.clone();
        let local_key = r.key.clone();
        let (mut ns, mut ch_sig) = (notice, channels);
        spawn(async move {
            // 行 key 优先（无外部改名竞态）；seed 演示行 key 为空才按名称找
            let key = if !local_key.is_empty() {
                local_key
            } else {
                match find_channel_by_name(&cur_name).await {
                    Ok(Some(c)) => c.key,
                    Ok(None) => {
                        ns.set(DrawerNotice::Err(format!(
                            "渠道「{cur_name}」不存在于后端（可能已被删除）"
                        )));
                        return;
                    }
                    Err(e) => {
                        ns.set(DrawerNotice::Err(format!("启停失败：{e}")));
                        return;
                    }
                }
            };
            match set_channel_status(&key, target as i16).await {
                Ok(()) => {
                    if let Some(r) = ch_sig.write().get_mut(i) {
                        r.status = target;
                    }
                    ns.set(DrawerNotice::Ok);
                    bump_topo_refresh();
                }
                Err(e) => ns.set(DrawerNotice::Err(format!("启停失败：{e}"))),
            }
        });
    };

    let mut request_delete = move |i: usize| {
        confirming.set(Some(i));
    };

    let confirm_delete = move |_| {
        let Some(i) = *confirming.peek() else {
            return;
        };
        confirming.set(None);
        let cur = channels.read().get(i).cloned();
        let Some(r) = cur else {
            return;
        };
        let local_key = r.key.clone();
        let (mut ns, mut ch_sig) = (notice, channels);
        spawn(async move {
            // 本地草稿行（后端不存在）直接删本地；真实渠道走 DELETE。
            // 行 key 优先（免查询且无改名竞态），seed 演示行为空才按名称找
            if !local_key.is_empty() {
                if let Err(e) = delete_channel(&local_key).await {
                    ns.set(DrawerNotice::Err(format!("删除失败：{e}")));
                    return;
                }
                ch_sig.write().remove(i);
                bump_topo_refresh();
                ns.set(DrawerNotice::Ok);
                return;
            }
            match find_channel_by_name(&r.name).await {
                Ok(Some(c)) => {
                    if let Err(e) = delete_channel(&c.key).await {
                        ns.set(DrawerNotice::Err(format!("删除失败：{e}")));
                        return;
                    }
                    ch_sig.write().remove(i);
                    bump_topo_refresh();
                    ns.set(DrawerNotice::Ok);
                }
                // 后端已无此名 → 本地行是幻影，只清本地
                Ok(None) => {
                    ch_sig.write().remove(i);
                    ns.set(DrawerNotice::Ok);
                }
                Err(e) => {
                    ns.set(DrawerNotice::Err(format!("删除失败：{e}")));
                }
            }
        });
    };

    rsx! {
        DrawerNoticeBar { notice, on_clear: move |_| notice.set(DrawerNotice::Idle) }
        CardPanel {
            section_index: 2,
            title: "渠道",
            hint: "URL + Key 是凭证容器；调度模型由候补池加入，名字不可改",
            count: channels.read().len(),
            open: open,
            on_toggle: on_toggle,

            div { class: "flex flex-wrap items-center gap-2",
                for (i, c) in channels.read().iter().enumerate() {
                    {
                        let label = c.name.clone();
                        let active = idx == i;
                        let tone = if active {
                            "border-zinc-100 bg-zinc-100 text-zinc-900"
                        } else {
                            "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                        };
                        rsx! {
                            button {
                                class: "rounded-full border px-3 py-1 text-xs font-medium transition-colors {tone}",
                                onclick: move |_| {
                                    current.set(i);
                                    load_row(i);
                                },
                                "{label}"
                            }
                        }
                    }
                }
                button {
                    class: "rounded-full border border-dashed border-zinc-700 px-3 py-1 text-xs text-zinc-500 hover:border-zinc-500 hover:text-zinc-300",
                    onclick: move |_| {
                        // 只把表单置成草稿态（is_new），不动 current——
                        // current 还指向已选渠道行，启停/删除按它定位，
                        // 草稿期这两个按钮与 NodeArea 一并禁用（下方）。
                        load_new(());
                    },
                    "＋ 新建渠道"
                }
            }

            div { class: "flex flex-wrap items-center gap-2",
                TextCell {
                    label: "渠道名称",
                    value: name(),
                    placeholder: "OpenAI 官方",
                    oninput: move |v: String| name.set(v),
                }
                SelectCell {
                    label: "类型",
                    value: ctype(),
                    options: crate::state::CHANNEL_TYPES,
                    oninput: move |v: String| ctype.set(v),
                }
                TextCell {
                    label: "Base URL",
                    value: url(),
                    placeholder: "https://…",
                    oninput: move |v: String| url.set(v),
                }
                TextCell {
                    label: "API Key（留空 = 不改动现有密钥）",
                    value: keys(),
                    placeholder: "sk-…",
                    oninput: move |v: String| keys.set(v),
                }
                button {
                    class: "rounded-md border border-zinc-100 bg-zinc-100 px-3 py-1.5 text-xs font-medium text-zinc-900 hover:bg-zinc-300",
                    disabled: saving(),
                    onclick: save,
                    "保存"
                }
                button {
                    class: "rounded-md border border-zinc-800 px-3 py-1.5 text-xs text-zinc-400 hover:border-zinc-600 hover:text-zinc-200",
                    disabled: is_new(),
                    title: if is_new() { "草稿未保存，保存后才能启停" } else { "" },
                    onclick: toggle_status,
                    if status() == 1 { "停用" } else { "启用" }
                }
                button {
                    class: "rounded-md border border-zinc-800 px-3 py-1.5 text-xs text-red-400 hover:border-red-700",
                    disabled: is_new(),
                    title: if is_new() { "草稿未保存，保存后才能删除" } else { "" },
                    onclick: move |_| request_delete(idx),
                    "删除此渠道"
                }
            }

            if is_new() {
                NodeArea {
                    EmptyHint { text: "草稿渠道：填好 URL + Key 保存后，这里才会显示候补池与调度模型" }
                }
            } else if let Some(c) = row {
                NodeArea {
                    div { class: "grid min-h-0 grid-cols-1 gap-3 lg:grid-cols-2",
                        // 候补池
                        div { class: "flex min-h-0 flex-col gap-2 rounded-lg border border-dashed border-zinc-700 bg-zinc-950/60 p-3",
                            div { class: "flex items-center justify-between gap-2",
                                div {
                                    p { class: "text-xs text-zinc-300", "候补池" }
                                    p { class: "text-[11px] text-zinc-600", "拉取结果，尚未进入拓扑" }
                                }
                            }
                            if c.candidates.is_empty() {
                                EmptyHint { text: "点「拉取模型」获取候补" }
                            } else {
                                div { class: "min-h-0 flex-1 space-y-0.5 overflow-y-auto",
                                    for (j, (m, on)) in c.candidates.iter().enumerate() {
                                        {
                                            let label = m.clone();
                                            let checked = *on;
                                            rsx! {
                                                label { class: "flex cursor-pointer items-center gap-2 rounded px-2 py-1 hover:bg-zinc-900",
                                                    input {
                                                        r#type: "checkbox",
                                                        class: "accent-zinc-100",
                                                        checked: checked,
                                                        onchange: move |_| {
                                                            let mut w = channels.write();
                                                            let v = w[idx].candidates[j].1;
                                                            w[idx].candidates[j].1 = !v;
                                                        },
                                                    }
                                                    span { class: "font-mono text-xs text-zinc-400", "{label}" }
                                                }
                                            }
                                        }
                                    }
                                }
                                div { class: "flex gap-1.5 border-t border-zinc-800 pt-2",
                                    button {
                                        class: "rounded border border-zinc-100 bg-zinc-100 px-2 py-0.5 text-[11px] font-medium text-zinc-900 hover:bg-zinc-300",
                                        onclick: move |_| {
                                            let mut w = channels.write();
                                            let picked: Vec<String> = w[idx]
                                                .candidates
                                                .iter()
                                                .filter(|(_, on)| *on)
                                                .map(|(n, _)| n.clone())
                                                .collect();
                                            for p in &picked {
                                                if !w[idx].dispatch.contains(p) {
                                                    w[idx].dispatch.push(p.clone());
                                                }
                                            }
                                            w[idx].candidates.retain(|(n, on)| !(*on && picked.contains(n)));
                                        },
                                        "加入调度 →"
                                    }
                                    button {
                                        class: "rounded border border-zinc-800 px-2 py-0.5 text-[11px] text-zinc-500 hover:border-zinc-600 hover:text-zinc-300",
                                        onclick: move |_| { channels.write()[idx].candidates.clear(); },
                                        "清空候补"
                                    }
                                }
                            }
                        }

                        // 调度模型
                        div { class: "flex min-h-0 flex-col gap-2 rounded-lg border border-zinc-800 bg-zinc-950 p-3",
                            div {
                                p { class: "text-xs text-zinc-300", "调度模型" }
                                p { class: "text-[11px] text-zinc-600", "已在拓扑中；名字来自上游，不可改" }
                            }
                            if c.dispatch.is_empty() {
                                EmptyHint { text: "从左侧候补池加入" }
                            } else {
                                div { class: "min-h-0 flex-1 space-y-1 overflow-y-auto",
                                    for (j, m) in c.dispatch.iter().enumerate() {
                                        {
                                            let label = m.clone();
                                            rsx! {
                                                div { class: "flex items-center justify-between gap-2 rounded border border-zinc-800 bg-zinc-900 px-2 py-1",
                                                    span { class: "truncate font-mono text-xs text-zinc-200", "{label}" }
                                                    button {
                                                        class: "shrink-0 text-[11px] text-zinc-600 hover:text-red-400",
                                                        title: "移出拓扑，退回候补池",
                                                        onclick: move |_| {
                                                            let mut w = channels.write();
                                                            let m = w[idx].dispatch.remove(j);
                                                            w[idx].candidates.push((m, false));
                                                        },
                                                        "✕"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 删除确认弹窗
        {
            let confirming_now = *confirming.peek();
            if let Some(ci) = confirming_now {
                let cname = channels.read().get(ci).map(|r| r.name.clone()).unwrap_or_default();
                rsx! {
                    Dialog {
                        title: "删除渠道".to_string(),
                        open: true,
                        on_confirm: confirm_delete,
                        on_cancel: move |_| confirming.set(None),
                        div { class: "text-xs text-zinc-400", "确认删除渠道「{cname}」？该操作直接生效于后端，不可撤销。" }
                    }
                }
            } else {
                rsx! { Fragment {} }
            }
        }
    }
}

// ============ 共享小件 ============

#[component]
fn CardPanel(
    section_index: usize,
    title: &'static str,
    hint: &'static str,
    count: usize,
    open: bool,
    on_toggle: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    let id = format!("ent-card-{section_index}");
    rsx! {
        section { id: "{id}", class: "shrink-0 overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900/60",
            button {
                class: "flex w-full items-center gap-2 px-4 py-2.5 text-left transition-colors hover:bg-zinc-900",
                onclick: move |e| on_toggle.call(e),
                span { class: "text-sm font-medium text-zinc-100", "{title}" }
                span { class: "rounded-full border border-zinc-700 px-1.5 text-[11px] text-zinc-400", "{count}" }
                span { class: "truncate text-[11px] text-zinc-600", "{hint}" }
            }
            if open {
                div { class: "space-y-3 border-t border-zinc-800 p-4", {children} }
            }
        }
    }
}

#[component]
fn NodeArea(children: Element) -> Element {
    rsx! {
        div { class: "min-h-[104px] rounded-lg border border-zinc-800 bg-zinc-950 p-3", {children} }
    }
}

#[component]
pub(crate) fn EmptyHint(text: &'static str) -> Element {
    rsx! {
        div { class: "flex h-full min-h-[72px] items-center justify-center",
            span { class: "text-[11px] text-zinc-600", "{text}" }
        }
    }
}

#[component]
pub(crate) fn EntityChip(
    label: String,
    sub: String,
    active: bool,
    on_pick: EventHandler<MouseEvent>,
    on_remove: EventHandler<MouseEvent>,
) -> Element {
    let tone = if active {
        "border-zinc-100 bg-zinc-100 text-zinc-900"
    } else {
        "border-zinc-700 bg-zinc-900 text-zinc-200 hover:border-zinc-500"
    };
    let sub_tone = if active {
        "text-zinc-600"
    } else {
        "text-zinc-500"
    };
    rsx! {
        span { class: "inline-flex items-center gap-1.5 rounded-full border py-1 pl-3 pr-1.5 transition-colors {tone}",
            button {
                class: "flex items-baseline gap-1.5",
                onclick: move |e| on_pick.call(e),
                span { class: "text-xs font-medium", "{label}" }
                if !sub.is_empty() {
                    span { class: "text-[11px] {sub_tone}", "{sub}" }
                }
            }
            button {
                class: "px-1 text-[11px] opacity-50 hover:text-red-400 hover:opacity-100",
                onclick: move |e| on_remove.call(e),
                "✕"
            }
        }
    }
}

#[component]
pub(crate) fn InputCell(
    label: &'static str,
    value: Signal<String>,
    placeholder: &'static str,
    #[props(default = false)] grow: bool,
) -> Element {
    let width = if grow { "min-w-[140px] flex-1" } else { "" };
    rsx! {
        label { class: "block space-y-1 {width}",
            span { class: "text-[11px] text-zinc-500", "{label}" }
            input {
                class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
                value: "{value.read()}",
                placeholder: "{placeholder}",
                oninput: move |e| value.set(e.value()),
            }
        }
    }
}

/// 原生下拉(移动端友好;样式与 InputCell/TextCell 一致)
#[component]
pub(crate) fn SelectCell(
    label: &'static str,
    value: String,
    options: &'static [&'static str],
    oninput: EventHandler<String>,
) -> Element {
    rsx! {
        label { class: "block space-y-1",
            span { class: "text-[11px] text-zinc-500", "{label}" }
            select {
                class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors focus:border-zinc-500",
                value: "{value}",
                oninput: move |e| oninput.call(e.value()),
                for opt in options {
                    option { value: "{opt}", selected: *opt == value, "{opt}" }
                }
            }
        }
    }
}

#[component]
pub(crate) fn TextCell(
    label: &'static str,
    value: String,
    placeholder: &'static str,
    oninput: EventHandler<String>,
) -> Element {
    rsx! {
        label { class: "block space-y-1",
            span { class: "text-[11px] text-zinc-500", "{label}" }
            input {
                class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
                value: "{value}",
                placeholder: "{placeholder}",
                oninput: move |e| oninput.call(e.value()),
            }
        }
    }
}
