//! 实体设置页的渠道卡:渠道列表 / 选中编辑 / 启停 / 删除,以及候补池 ↔ 调度模型
//! 的双向搬运。
//!
//! 负责:渠道的录入与保存(新建走 `create_channel_import`,编辑走 `update_channel`)、
//! 启停(`set_channel_status`)、删除(`delete_channel`),以及候补池/调度模型的本地
//! 勾选与搬运。
//!
//! 不负责:分组卡与别名卡(`cards.rs`)、卡片折叠态(`page.rs`)、写路径实现
//! (`crate::drawer_write`)、拓扑画布刷新(`bump_topo_refresh` 只被调用)。
//!
//! 本卡是 RENAMED 来的(原 `entities.rs` 的渠道部分):候补池/调度模型操作纯本地,
//! 不落库;只有渠道本身的 CRUD 与启停会发请求。

use super::shared::*;
use crate::drawer_write::{
    DrawerNotice, DrawerNoticeBar, create_channel_import, delete_channel, find_channel_by_name,
    set_channel_status, update_channel,
};
use crate::state::EntityStore;
use crate::tab_page_network::bump_topo_refresh;
use dioxus::prelude::*;
use ui::dialog::Dialog;

// ============ 卡片 3：渠道 ============

/// 渠道卡：渠道的录入 / 启停 / 删除与候补池 ↔ 调度模型搬运。
///
/// 【是什么】实体设置页的第三张卡:渠道胶囊列表 + 录入行 + 节点区(左候补池、右调度模型)
/// + 删除确认弹窗,外加顶部写操作提示条。
///
/// 【做什么】负责渠道的保存(新建 `create_channel_import` / 编辑 `update_channel`)、
/// 启停(`set_channel_status`)、删除(`delete_channel`),并在成功后
/// `bump_topo_refresh()` 刷拓扑画布。候补池的勾选、加入调度、清空、移出拓扑全部
/// 只改本地 store 行,不落库。不负责分组/别名卡与折叠态。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点渠道胶囊 → `load_row(i)` 回填录入行(keys 固定留空 = 不覆盖现网密钥)。
/// - 点「＋ 新建渠道」→ `load_new` 把表单置为草稿态(不动 `current`,故启停/删除被禁用)。
/// - 点「保存」→ `save`:先拦空名与「新建无 Key」,新建 POST 后 push 本地行并切到新行;
///   编辑优先用行内服务端 key(无改名竞态),seed 行才 `find_channel_by_name` 兜底,
///   失败一律报错而不降级成新建(避免复制出重复渠道);`saving` 为真时防双击。
/// - 点「停用/启用」→ `toggle_status`(1↔2),成功后写回本地行 `status`。
/// - 点「删除此渠道」→ 打开确认弹窗;确认后本地 key 存在走 `delete_channel`,
///   否则按名称查找,后端已无此名时只清本地行。
/// - 候补池内勾选/「加入调度 →」/「清空候补」/ 调度模型行的 ✕ 只操作本地 store 行,无网络。
///
/// 【样式】外壳与头部由 `CardPanel` 提供;胶囊行 `flex flex-wrap items-center gap-2`,
/// 选中态 `border-zinc-100 bg-zinc-100 text-zinc-900`;录入行同为 `flex flex-wrap
/// items-center gap-2`;节点区为 `grid grid-cols-1 gap-3 lg:grid-cols-2`,左格
/// `border-dashed border-zinc-700 bg-zinc-950/60`,右格实线 `border-zinc-800 bg-zinc-950`;
/// 删除确认用 `ui::dialog::Dialog`。
///
/// 【子组件组成】`DrawerNoticeBar`(写操作提示条)、`CardPanel`(卡外壳)、
/// `TextCell`(渠道名/Base URL/API Key)、`SelectCell`(渠道类型)、`NodeArea`(节点槽)、
/// `EmptyHint`(三处空态)、`Dialog`(删除确认,条件渲染)。
///
/// 【数据流】
/// - 对内(入):`open`(卡片展开态,页面持有)、`on_toggle`(切展开);渠道数据来自
///   `use_context::<EntityStore>()` 的 `channels`(本地 store 行,仅作拓扑启动布局兜底)。
/// - 对外(出):写本地 `channels` 行、置 `notice`、调 `bump_topo_refresh()` 通知画布重拉;
///   `on_toggle` 冒泡给页面翻转 `open` 数组。
#[component]
pub fn ChannelsCard(open: bool, on_toggle: EventHandler<MouseEvent>) -> Element {
    let store = use_context::<EntityStore>();
    let mut channels = store.channels;
    // —— 卡片自身状态:选中下标、录入行、草稿标记、提示与确认 ——
    // 这组状态只服务本卡,不跨组件,故就地持有(channels 行本身在 EntityStore)。
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
        name.set(MSG_NEW_CHANNEL_NAME.into());
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
            notice.set(DrawerNotice::Err(MSG_ERR_NEED_API_KEY.into()));
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
                        ns.set(DrawerNotice::Err(format!("{MSG_SAVE_FAILED_PREFIX}{e}")));
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
                    dispatch: crate::tab_page_network::channel_models(&created.models),
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
                                ns.set(DrawerNotice::Err(format!("{MSG_SAVE_FAILED_PREFIX}{e}")));
                                return;
                            }
                        },
                        Err(e) => {
                            ns.set(DrawerNotice::Err(format!("{MSG_SAVE_FAILED_PREFIX}{e}")));
                            return;
                        }
                    }
                };
                // 用户未重输 keys → 传 None（请求体缺席 keys 字段 = 保留现值）
                let new_keys = (!kvec.is_empty()).then(|| kvec.clone());
                if let Err(e) = update_channel(&key, &n, &u, new_keys).await {
                    ns.set(DrawerNotice::Err(format!("{MSG_SAVE_FAILED_PREFIX}{e}")));
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
                Err(e) => ns.set(DrawerNotice::Err(format!("{MSG_SAVE_FAILED_PREFIX}{e}"))),
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
                            "{MSG_CHANNEL_MISSING_PREFIX}{cur_name}{MSG_CHANNEL_MISSING_SUFFIX}"
                        )));
                        return;
                    }
                    Err(e) => {
                        ns.set(DrawerNotice::Err(format!("{MSG_TOGGLE_FAILED_PREFIX}{e}")));
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
                Err(e) => ns.set(DrawerNotice::Err(format!("{MSG_TOGGLE_FAILED_PREFIX}{e}"))),
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
                    ns.set(DrawerNotice::Err(format!("{MSG_DELETE_FAILED_PREFIX}{e}")));
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
                        ns.set(DrawerNotice::Err(format!("{MSG_DELETE_FAILED_PREFIX}{e}")));
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
                    ns.set(DrawerNotice::Err(format!("{MSG_DELETE_FAILED_PREFIX}{e}")));
                }
            }
        });
    };

    rsx! {
        DrawerNoticeBar { notice, on_clear: move |_| notice.set(DrawerNotice::Idle) }
        CardPanel {
            section_index: 2,
            title: SEC_CARD_CHANNELS,
            hint: SEC_CARD_CHANNELS_HINT,
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
                    class: "rounded-full border border-dashed border-zinc-700 px-3 py-1 {ui::TYPE_DESC} hover:border-zinc-500 hover:text-zinc-300",
                    onclick: move |_| {
                        // 只把表单置成草稿态（is_new），不动 current——
                        // current 还指向已选渠道行，启停/删除按它定位，
                        // 草稿期这两个按钮与 NodeArea 一并禁用（下方）。
                        load_new(());
                    },
                    "{BTN_NEW_CHANNEL}"
                }
            }

            div { class: "flex flex-wrap items-center gap-2",
                TextCell {
                    label: FIELD_CHANNEL_NAME,
                    value: name(),
                    placeholder: MSG_PH_CHANNEL_NAME,
                    oninput: move |v: String| name.set(v),
                }
                SelectCell {
                    label: FIELD_CHANNEL_TYPE,
                    value: ctype(),
                    options: crate::state::CHANNEL_TYPES,
                    oninput: move |v: String| ctype.set(v),
                }
                TextCell {
                    label: FIELD_BASE_URL,
                    value: url(),
                    placeholder: MSG_PH_BASE_URL,
                    oninput: move |v: String| url.set(v),
                }
                TextCell {
                    label: FIELD_API_KEY,
                    value: keys(),
                    placeholder: MSG_PH_API_KEY,
                    oninput: move |v: String| keys.set(v),
                }
                button {
                    class: "rounded-md border border-zinc-100 bg-zinc-100 px-3 py-1.5 text-xs font-medium text-zinc-900 hover:bg-zinc-300",
                    disabled: saving(),
                    onclick: save,
                    "{BTN_SAVE}"
                }
                button {
                    class: "rounded-md border border-zinc-800 px-3 py-1.5 text-xs text-zinc-400 hover:border-zinc-600 hover:text-zinc-200",
                    disabled: is_new(),
                    title: if is_new() { MSG_TITLE_DRAFT_NO_TOGGLE } else { "" },
                    onclick: toggle_status,
                    if status() == 1 { "{BTN_DISABLE}" } else { "{BTN_ENABLE}" }
                }
                button {
                    class: "rounded-md border border-zinc-800 px-3 py-1.5 text-xs {ui::C_DANGER} hover:border-red-700",
                    disabled: is_new(),
                    title: if is_new() { MSG_TITLE_DRAFT_NO_DELETE } else { "" },
                    onclick: move |_| request_delete(idx),
                    "{BTN_DELETE_CHANNEL}"
                }
            }

            if is_new() {
                NodeArea {
                    EmptyHint { text: MSG_DRAFT_CHANNEL }
                }
            } else if let Some(c) = row {
                NodeArea {
                    div { class: "grid min-h-0 grid-cols-1 gap-3 lg:grid-cols-2",
                        // 候补池
                        div { class: "flex min-h-0 flex-col gap-2 rounded-lg border border-dashed border-zinc-700 bg-zinc-950/60 p-3",
                            div { class: "flex items-center justify-between gap-2",
                                div {
                                    p { class: "text-xs text-zinc-300", "{LBL_CANDIDATE_POOL}" }
                                    p { class: "text-[11px] text-zinc-600", "{LBL_CANDIDATE_POOL_HINT}" }
                                }
                            }
                            if c.candidates.is_empty() {
                                EmptyHint { text: MSG_EMPTY_CANDIDATES }
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
                                        "{BTN_JOIN_DISPATCH}"
                                    }
                                    button {
                                        class: "rounded border border-zinc-800 px-2 py-0.5 text-[11px] text-zinc-500 hover:border-zinc-600 hover:text-zinc-300",
                                        onclick: move |_| { channels.write()[idx].candidates.clear(); },
                                        "{BTN_CLEAR_CANDIDATES}"
                                    }
                                }
                            }
                        }

                        // 调度模型
                        div { class: "flex min-h-0 flex-col gap-2 rounded-lg border border-zinc-800 bg-zinc-950 p-3",
                            div {
                                p { class: "text-xs text-zinc-300", "{LBL_DISPATCH_MODELS}" }
                                p { class: "text-[11px] text-zinc-600", "{LBL_DISPATCH_MODELS_HINT}" }
                            }
                            if c.dispatch.is_empty() {
                                EmptyHint { text: MSG_EMPTY_DISPATCH }
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
                                                        title: MSG_TITLE_MOVE_OUT,
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
                        title: TTL_DELETE_CHANNEL.to_string(),
                        open: true,
                        on_confirm: confirm_delete,
                        on_cancel: move |_| confirming.set(None),
                        div { class: "text-xs text-zinc-400", "{MSG_CONFIRM_DELETE_CHANNEL_PREFIX}{cname}{MSG_CONFIRM_DELETE_SUFFIX}" }
                    }
                }
            } else {
                rsx! { Fragment {} }
            }
        }
    }
}
