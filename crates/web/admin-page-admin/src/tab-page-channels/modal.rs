//! 渠道新建/编辑综合弹窗。纯渲染 + 就地提交:表单状态以 `Signal` 注入,
//! 保存成功走 `on_submit`(关弹窗 + 重拉列表),失败保持打开并内嵌展示
//! `channel-save-error`(role=alert)供就地重试。
//!
//! 边界:只有本弹窗的**提交体构造与保存请求**在这里(编辑发 `UpdateChannelBody`
//! 最小 diff,新建发全量 `ChannelUpsertRequest`);分组候选与密钥掩码的拉取、
//! 以及表单初值回填都在 `page.rs`。选择器外壳复用 `tab_page_groups::Modal`。

use dioxus::prelude::*;
use serde_json::json;

use client::ApiClient;
use contract::api::admin::{ChannelUpsertRequest, GroupDto};

use crate::api::{
    UpdateChannelBody, create_channel_api, fetch_channel_models_api, update_channel_api,
};
use crate::state::CHANNEL_TYPES;
use crate::tab_page_groups::Modal;

use super::shared::{
    BTN_CANCEL, BTN_CREATE_CHANNEL, BTN_SAVE_CHANGES, FIELD_API_KEY, FIELD_BASE_URL,
    FIELD_BOUND_GROUPS, FIELD_CHANNEL_NAME, FIELD_CHANNEL_TYPE, FIELD_REMARK,
    MSG_EXISTING_KEYS_PREFIX, MSG_EXISTING_KEYS_SUFFIX, MSG_FETCH_MODELS, MSG_FETCH_MODELS_FAILED,
    MSG_FETCH_MODELS_HINT, MSG_FETCHING_MODELS, MSG_GROUP_ERR_MID, MSG_GROUP_ERR_PREFIX,
    MSG_NO_GROUPS, MSG_NO_MODEL_CANDIDATES, MSG_PH_API_KEY_EDIT, MSG_PH_API_KEY_NEW,
    MSG_PH_BASE_URL, MSG_PH_CHANNEL_NAME, MSG_PH_REMARK, MSG_SAVE_FAILED, TTL_EDIT, TTL_NEW,
    parse_keys_input,
};
/// 渠道编辑/新建综合弹窗 (含类型、名称、URL、Key、分组、备注;后端暂不支持模型调度候补)。
/// 编辑分支发 [`UpdateChannelBody`] 最小 diff 体;新建分支仍用全量
/// [`ChannelUpsertRequest`]（创建语义要求 keys/models 等字段必须给全）。
/// 保存成功走 `on_submit`（关弹窗+重拉列表），失败弹窗保持打开并内嵌展示
/// `channel-save-error`（role=alert）供就地重试。
///
/// 【是什么】渠道 tab 的新建/编辑弹窗:类型下拉、分组多选 chips、名称/URL/Key/备注
/// 表单,编辑态额外带密钥掩码只读区与「拉取上游模型」候选池。
///
/// 【做什么】渲染表单、就地提交(唯一直接发请求的区段组件)、内联展示保存失败;
/// 不负责弹窗开关(`modal_state` 归页面)、不负责分组候选与掩码的拉取(页面 effect
/// 与 `open_edit` 里做)。两次提交语义不同:编辑走最小 diff 体,新建走全量体。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 表单输入(类型 `onchange`、名称/URL/Key/备注 `oninput`)→ 写注入的 Signal,不发网络。
/// - 点分组 chip → 在 `group.read()` 基础上增删该分组名后 `group.set(...)`,不发网络。
/// - 模型勾选框 `onchange` → 翻转 `model_pool` 中该项的勾选位,并置
///   `models_touched = true`(标记用户动过面板,提交时才携带 `models`)。
/// - 点「拉取上游模型」(仅编辑态)→ `fetch_channel_models_api`,上游已有而池中没有的
///   模型默认勾选、池里已有的保持原勾选;失败写进 `submit_err` 内联展示。
/// - 点「取消」→ `on_cancel`(页面置 `Closed`)。
/// - 点「保存修改 / 创建渠道」→ 本地 `spawn` 直接提交:编辑发 `UpdateChannelBody`
///   (`keys` 留空则字段整体缺席以保持现有密钥,`test_model` 恒带现值,
///   `models` 仅在 `models_touched` 时携带),新建发全量 `ChannelUpsertRequest`;
///   成功才调 `on_submit`,失败保持在弹窗内(role=alert)供重试。
///
/// 【样式】外壳复用 `Modal`;内容区 `space-y-4 max-h-[70vh] overflow-y-auto pr-1`,
/// 类型与分组两栏 `grid grid-cols-2 gap-3`;分组多选 chips:选中为绿调
/// `border-emerald-500/60 bg-emerald-500/15 text-emerald-300`,未选为描边
/// `border-zinc-700 bg-zinc-900`;模型候选池 `max-h-40 overflow-y-auto` 描边面板;
/// 错误块 `rounded-xl border-red-500/30 bg-red-950/30 text-red-400`;底部
/// `mt-6 flex gap-3`,取消描边、提交白底且 `disabled:opacity-40`。
///
/// 【子组件组成】`Modal`(弹窗外壳);其余均为原生元素(select / input / textarea /
/// 分组 chip 按钮 / 模型勾选行),未再拆子组件。
///
/// 【数据流】
/// - 对内(入):`editing`(决定标题与提交文案,并决定是否显示掩码区与模型面板)、
///   `channel_key`(编辑态的渠道 UUID,新建为 `None`);表单 Signal —— `name` / `ctype` /
///   `url` / `keys` / `group` / `remark` / `test_model`(`test_model` 无编辑控件,
///   仅原样回传,缺席会被后端清成 NULL);弹窗数据 Signal —— `group_options` /
///   `group_err` / `existing_keys`(掩码,只读)/ `model_pool` / `models_touched` /
///   `fetching_models`;回调 `on_cancel` / `on_submit`。全部由 `page.rs` 持有。
/// - 对外(出):所有表单 Signal 就地写回页面状态;`submit_err` / `submitting` 是
///   组件内部状态(每次关闭重挂载自然复位);`on_submit` 抛回页面 `close_and_reload`
///   (关弹窗 + `reload + 1` 触发列表重拉);拉取模型、创建、更新三个请求在本组件内发出。
#[component]
pub fn ChannelFormModal(
    editing: bool,
    channel_key: Option<String>,
    name: Signal<String>,
    ctype: Signal<String>,
    url: Signal<String>,
    keys: Signal<String>,
    group: Signal<Vec<String>>,
    group_options: Signal<Vec<GroupDto>>,
    group_err: Signal<Option<String>>,
    existing_keys: Signal<Vec<String>>,
    model_pool: Signal<Vec<(String, bool)>>,
    models_touched: Signal<bool>,
    fetching_models: Signal<bool>,
    remark: Signal<String>,
    test_model: Signal<Option<String>>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let title = if editing { TTL_EDIT } else { TTL_NEW };
    // 分组候选拉取失败/为空时的只读回退展示串（rsx 内不能嵌 let 语句）
    let bound_groups = group.read().join(", ");
    // chips 渲染数据（rsx 内不能嵌 let）：（分组名, 展示名, 是否已选）
    let group_chip_data: Vec<(String, String, bool)> = group_options
        .read()
        .iter()
        .map(|g| {
            let label = if g.remark.is_empty() {
                g.name.clone()
            } else {
                g.remark.clone()
            };
            let selected = group.read().contains(&g.name);
            (g.name.clone(), label, selected)
        })
        .collect();
    let submit_label = if editing {
        BTN_SAVE_CHANGES
    } else {
        BTN_CREATE_CHANNEL
    };

    let submitting = use_signal(|| false);
    // 保存失败信息（新建/编辑两条路径共用）：非空时弹窗保持打开、
    // 内嵌展示错误供用户就地重试；弹窗关闭重挂载时自然复位。
    let submit_err = use_signal(|| None::<String>);

    // 工厂式复制,避免把原 signal 移动出闭包(供 rsx 中 submitting() 继续读取)
    let submitting2 = submitting;
    let submit_err2 = submit_err;
    let on_submit2 = on_submit;
    let channel_key2 = channel_key.clone();
    let group2 = group;
    let model_pool2 = model_pool;
    let models_touched2 = models_touched;
    let do_submit = move |_| {
        let key = channel_key2.clone();
        let n = name.peek().trim().to_string();
        if n.is_empty() {
            return;
        }
        let ct = ctype.peek().clone();
        let u = url.peek().trim().to_string();
        let k = parse_keys_input(&keys.peek());
        let gvec = group2.peek().clone();
        let rm = remark.peek().clone();
        let tm = test_model.peek().clone();
        let (mut sub, mut serr, cb) = (submitting2, submit_err2, on_submit2);
        spawn(async move {
            sub.set(true);
            serr.set(None); // 新一轮尝试，清掉上一次的失败提示
            let client = ApiClient::shared().clone();

            let res = match key {
                // 编辑:最小 diff 体——keys 未重输则字段整体缺席(保持现有密钥,
                // 恒发 [] 会被后端 400 拒绝);testModel 恒带现值(直绑列,缺席即清);
                // models/priority/weight 等弹窗不管理的列不发(COALESCE 保持)。
                Some(kk) => {
                    let body = UpdateChannelBody {
                        name: n,
                        channel_type: ct,
                        base_url: u,
                        groups: gvec,
                        remark: rm,
                        test_model: tm,
                        keys: (!k.is_empty()).then_some(k),
                        // 仅当用户动过「拉取模型」面板才携带 models（勾选集整体
                        // 替换该列）；未动 = 缺席 = 后端 COALESCE 保持现值。
                        models: (*models_touched2.peek()).then(|| {
                            serde_json::Value::Array(
                                model_pool2
                                    .read()
                                    .iter()
                                    .filter(|(_, checked)| *checked)
                                    .map(|(id, _)| {
                                        // validate 硬要求：每条须非空 alias+upstream，
                                        // 裸字符串数组会被 400 拒绝。v1 语义：对外名 = 上游名
                                        serde_json::json!({
                                            "alias": id.clone(),
                                            "upstream": id.clone(),
                                        })
                                    })
                                    .collect(),
                            )
                        }),
                    };
                    update_channel_api(&client, &kk, &body).await
                }
                None => {
                    let req = ChannelUpsertRequest {
                        name: n,
                        channel_type: ct,
                        base_url: u,
                        keys: k,
                        models: json!([]),
                        groups: gvec,
                        priority: 0,
                        weight: 0,
                        test_model: tm,
                        remark: rm,
                    };
                    create_channel_api(&client, &req).await
                }
            };
            sub.set(false);
            match res {
                // 成功才走 on_submit（关弹窗 + 重拉列表）；失败保持弹窗打开、
                // 错误就地展示——此前 `let _ = res;` 把失败吞成静默假成功。
                Ok(_) => cb.call(()),
                Err(e) => serr.set(Some(format!("{MSG_SAVE_FAILED}{e}"))),
            }
        });
    };

    rsx! {
        Modal { title: title.to_string(), on_close: move |_| on_cancel.call(()),
            div { class: "space-y-4 max-h-[70vh] overflow-y-auto pr-1",
                div { class: "grid grid-cols-2 gap-3",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_CHANNEL_TYPE}" }
                        select {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3 py-2 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                            value: "{ctype}",
                            onchange: move |e| ctype.set(e.value()),
                            for opt in CHANNEL_TYPES {
                                option { value: "{opt}", "{opt}" }
                            }
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_BOUND_GROUPS}" }
                        div { class: "flex min-h-[38px] flex-wrap items-center gap-1.5 rounded-xl border border-zinc-700 bg-zinc-950 px-2 py-1.5",
                            if group_options.read().is_empty() {
                                // 候选拉取失败/为空：只读展示当前已绑分组，不阻断保存
                                span { class: "{ui::TYPE_DESC}",
                                    if let Some(e) = group_err.read().as_ref() {
                                        "{MSG_GROUP_ERR_PREFIX}{e}{MSG_GROUP_ERR_MID}{bound_groups}"
                                    } else if group.read().is_empty() {
                                        "{MSG_NO_GROUPS}"
                                    } else {
                                        "{bound_groups}"
                                    }
                                }
                            } else {
                                for (gname, glabel, selected) in group_chip_data.iter().cloned() {
                                    button {
                                        key: "{gname}",
                                        class: if selected {
                                            "rounded-full border border-emerald-500/60 bg-emerald-500/15 px-2.5 py-0.5 text-xs font-medium text-emerald-300"
                                        } else {
                                            "rounded-full border border-zinc-700 bg-zinc-900 px-2.5 py-0.5 text-xs text-zinc-400 hover:border-zinc-500 hover:text-zinc-200"
                                        },
                                        onclick: move |_| {
                                            let mut cur = group.read().clone();
                                            if cur.contains(&gname) {
                                                cur.retain(|x| x != &gname);
                                            } else {
                                                cur.push(gname.clone());
                                            }
                                            group.set(cur);
                                        },
                                        "{glabel}"
                                    }
                                }
                            }
                        }
                    }
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_CHANNEL_NAME}" }
                    input {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                        placeholder: "{MSG_PH_CHANNEL_NAME}",
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_BASE_URL}" }
                    input {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                        placeholder: "{MSG_PH_BASE_URL}",
                        value: "{url}",
                        oninput: move |e| url.set(e.value()),
                    }
                }

                if editing && !existing_keys.read().is_empty() {
                    // 掩码只读展示：明文永不出后端（单查接口也回掩码）。
                    // 独立于下方 textarea，物理隔离保证掩码串不可能进入提交体。
                    div { class: "space-y-1",
                        span { class: "block text-[11px] text-zinc-500",
                            "{MSG_EXISTING_KEYS_PREFIX}{existing_keys.read().len()}{MSG_EXISTING_KEYS_SUFFIX}"
                        }
                        for mk in existing_keys.read().iter() {
                            div { class: "rounded-md border border-zinc-800 bg-zinc-900/60 px-2.5 py-1 font-mono text-xs text-zinc-400", "{mk}" }
                        }
                    }
                }
                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_API_KEY}" }
                    textarea {
                        class: "w-full h-20 rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none resize-none",
                        placeholder: if editing {
                            "{MSG_PH_API_KEY_EDIT}"
                        } else {
                            "{MSG_PH_API_KEY_NEW}"
                        },
                        value: "{keys}",
                        oninput: move |e| keys.set(e.value()),
                    }
                }

                if editing {
                    div { class: "space-y-1.5",
                        div { class: "flex items-center gap-2",
                            button {
                                class: if *fetching_models.read() {
                                    "rounded-lg border border-zinc-700 bg-zinc-800/60 px-3 py-1.5 text-xs text-zinc-500"
                                } else {
                                    "rounded-lg border border-zinc-700 bg-zinc-800/60 px-3 py-1.5 text-xs text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white"
                                },
                                disabled: *fetching_models.read(),
                                onclick: move |_| {
                                    let Some(k) = channel_key.clone() else { return };
                                    if *fetching_models.peek() {
                                        return;
                                    }
                                    fetching_models.set(true);
                                    let mut pool = model_pool;
                                    let mut touched = models_touched;
                                    let mut flag = fetching_models;
                                    let mut serr = submit_err;
                                    spawn(async move {
                                        let client = ApiClient::shared().clone();
                                        flag.set(true);
                                        match fetch_channel_models_api(&client, &k).await {
                                            Ok(ids) => {
                                                let mut cur = pool.read().clone();
                                                let known: std::collections::HashSet<String> =
                                                    cur.iter().map(|(id, _)| id.clone()).collect();
                                                for id in ids {
                                                    // 上游已有、池里没有的模型默认勾选；池里已有的保持用户勾选状态
                                                    if !known.contains(&id) {
                                                        cur.push((id, true));
                                                    }
                                                }
                                                pool.set(cur);
                                                touched.set(true);
                                            }
                                            Err(e) => {
                                                // 弹窗内联展示（channel-save-error 区，role=alert）
                                                serr.set(Some(format!("{MSG_FETCH_MODELS_FAILED}{e}")));
                                            }
                                        }
                                        flag.set(false);
                                    });
                                },
                                if *fetching_models.read() { "{MSG_FETCHING_MODELS}" } else { "{MSG_FETCH_MODELS}" }
                            }
                            span { class: "text-[11px] text-zinc-500",
                                "{MSG_FETCH_MODELS_HINT}"
                            }
                        }
                        div { class: "max-h-40 overflow-y-auto rounded-xl border border-zinc-700 bg-zinc-950 p-2 space-y-1",
                            if model_pool.read().is_empty() {
                                span { class: "text-[11px] text-zinc-600", "{MSG_NO_MODEL_CANDIDATES}" }
                            } else {
                                for (idx, (id, checked)) in model_pool.read().iter().enumerate() {
                                    label { class: "flex items-center gap-2 rounded-md px-1.5 py-0.5 hover:bg-zinc-900",
                                        input {
                                            r#type: "checkbox",
                                            checked: *checked,
                                            onchange: move |_| {
                                                let mut cur = model_pool.read().clone();
                                                if let Some(entry) = cur.get_mut(idx) {
                                                    entry.1 = !entry.1;
                                                }
                                                model_pool.set(cur);
                                                models_touched.set(true);
                                            },
                                        }
                                        span { class: "font-mono text-xs text-zinc-300", "{id}" }
                                    }
                                }
                            }
                        }
                    }
                }
                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "{FIELD_REMARK}" }
                    textarea {
                        class: "w-full h-16 rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none resize-none",
                        placeholder: "{MSG_PH_REMARK}",
                        value: "{remark}",
                        oninput: move |e| remark.set(e.value()),
                    }
                }
            }

            // 保存失败提示（复用 system.rs 表单错误块样式与 alert 角色），
            // 紧贴操作按钮上方，用户看到错误后可直接改参重试
            if let Some(msg) = submit_err() {
                div {
                    role: "alert",
                    class: "rounded-xl border border-red-500/30 bg-red-950/30 p-4 text-sm {ui::STATE_DANGER_TEXT}",
                    "data-testid": "channel-save-error",
                    "{msg}"
                }
            }

            div { class: "mt-6 flex gap-3",
                button {
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                    onclick: move |_| on_cancel.call(()),
                    "{BTN_CANCEL}"
                }
                button {
                    class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200 disabled:opacity-40",
                    disabled: submitting(),
                    onclick: do_submit,
                    "{submit_label}"
                }
            }
        }
    }
}
