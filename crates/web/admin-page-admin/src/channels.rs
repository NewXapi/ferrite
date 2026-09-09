//! 渠道管理页:卡片式网格,对齐 GroupsPage / UsersPanel 规范。
//! 数据来自真实后端:挂载时 `use_effect` 拉 `list_channels_api`,写入本地
//! `channels` signal;启用/停用走 `set_channel_status_api`,删除走
//! `delete_channel_api`,新建/编辑走 `create_channel_api` / `update_channel_api`。
//! 拓扑测速、批量分组、快速导入是 mock 期的纯前端特性,已移除(后端暂无对应接口)。

use dioxus::prelude::*;
use serde_json::json;
use ui::SegmentedCapsule;

use client::ApiClient;
use contract::api::admin::{ChannelDto, ChannelUpsertRequest};

use crate::api::{
    create_channel_api, delete_channel_api, list_channels_api, set_channel_status_api,
    update_channel_api,
};
use crate::groups::{Badge, Modal, StatCard};
use crate::state::CHANNEL_TYPES;

/// 弹窗状态
#[derive(Clone, PartialEq)]
enum ChannelModalState {
    Closed,
    New,
    Edit(String),
}

/// 写操作种类:在写回工厂里区分启停与删除
#[derive(Clone, Copy)]
enum WriteOp {
    Toggle(i16),
    Delete,
}

const SEC_STATS: &str = "渠道概览";
const SEC_FILTER: &str = "筛选与操作";
const SEC_LIST: &str = "渠道列表";

#[component]
pub fn ChannelsPage() -> Element {
    // 真实数据 + 加载/错误态(本地 signal,不触碰 EntityStore)
    let mut channels = use_signal(Vec::<ChannelDto>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    // 写操作进行中 / 成功提示
    let busy = use_signal(|| false);
    let notice = use_signal(|| None::<String>);
    // reload 计数:触发一次即重拉列表(写操作后刷新)
    let mut reload = use_signal(|| 0u32);

    let mut search = use_signal(String::new);
    let mut filter_tier = use_signal(|| 0usize);
    let mut modal_state = use_signal(|| ChannelModalState::Closed);

    // 编辑/新建表单状态
    let mut f_name = use_signal(String::new);
    let mut f_ctype = use_signal(|| "openai".to_string());
    let mut f_url = use_signal(String::new);
    let mut f_keys = use_signal(String::new);
    let mut f_group = use_signal(|| "default".to_string());
    let mut f_remark = use_signal(String::new);

    // 挂载即拉取真实列表;reload 变化时重拉
    use_effect(move || {
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            match list_channels_api(&client).await {
                Ok(list) => {
                    channels.set(list);
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    let list = channels();
    let total = list.len();
    let enabled_count = list.iter().filter(|c| c.status == 1).count();
    let disabled_count = list.iter().filter(|c| c.status != 1).count();
    let total_keys: i64 = list.iter().map(|c| c.key_count).sum();
    let group_set: std::collections::BTreeSet<String> =
        list.iter().map(|c| c.group_name.clone()).collect();

    let stats: [(String, &str); 5] = [
        (total.to_string(), "总渠道数"),
        (enabled_count.to_string(), "正常启用"),
        (disabled_count.to_string(), "停用/异常"),
        (total_keys.to_string(), "密钥总数"),
        (group_set.len().to_string(), "绑定分组数"),
    ];

    let filter_options = vec![
        format!("全部 ({total})"),
        format!("启用中 ({enabled_count})"),
        format!("已停用 ({disabled_count})"),
    ];

    let filtered: Vec<ChannelDto> = {
        let q = search().trim().to_lowercase();
        let tier = filter_tier();
        list.into_iter()
            .filter(|c| {
                if !q.is_empty()
                    && !c.name.to_lowercase().contains(&q)
                    && !c.channel_type.to_lowercase().contains(&q)
                    && !c.base_url.to_lowercase().contains(&q)
                    && !c.group_name.to_lowercase().contains(&q)
                {
                    return false;
                }
                match tier {
                    1 => c.status == 1,
                    2 => c.status != 1,
                    _ => true,
                }
            })
            .collect()
    };

    let open_new = move |_| {
        f_name.set(String::new());
        f_ctype.set("openai".to_string());
        f_url.set("https://api.openai.com/v1".to_string());
        f_keys.set(String::new());
        f_group.set("default".to_string());
        f_remark.set(String::new());
        modal_state.set(ChannelModalState::New);
    };

    let mut open_edit = move |key: String| {
        if let Some(c) = channels().iter().find(|c| c.key == key) {
            f_name.set(c.name.clone());
            f_ctype.set(c.channel_type.clone());
            f_url.set(c.base_url.clone());
            f_keys.set(String::new());
            f_group.set(c.group_name.clone());
            f_remark.set(c.remark.clone());
            modal_state.set(ChannelModalState::Edit(key));
        }
    };

    // 写操作助手工厂:返回独立闭包,分别交给卡片(启停/删除)。
    // Signal 是 Copy;每次调用先复制一份再 move 进 async,避免把闭包捕获的
    // signal 移动出去(FnMut 不允许)。
    let make_write = || {
        let busy_sig = busy;
        let notice_sig = notice;
        let reload_sig = reload;
        move |key: String, op: WriteOp| {
            let (mut b, mut n, mut r) = (busy_sig, notice_sig, reload_sig);
            spawn(async move {
                b.set(true);
                n.set(None);
                let client = ApiClient::shared().clone();
                let res = match op {
                    WriteOp::Toggle(status) => set_channel_status_api(&client, &key, status).await,
                    WriteOp::Delete => delete_channel_api(&client, &key).await,
                };
                match res {
                    Ok(_) => {
                        n.set(Some("操作成功".to_string()));
                        r.set(r() + 1);
                    }
                    Err(e) => n.set(Some(format!("操作失败:{e}"))),
                }
                b.set(false);
            });
        }
    };
    let write_toggle = make_write();
    let write_delete = make_write();

    // 弹窗关闭并触发重拉
    let close_and_reload = move |_| {
        modal_state.set(ChannelModalState::Closed);
        reload.set(reload() + 1);
    };

    rsx! {
        div { class: "flex flex-col gap-6",
                // 通知条(成功/错误/进行中)
                if let Some(msg) = notice() {
                    div { class: "rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2 text-xs text-zinc-300",
                        "{msg}"
                        if busy() { " ···" }
                    }
                }

                // 1. 概览统计区
                section { id: "channels-sec-stats", class: "scroll-mt-8 space-y-3",
                    h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
                    div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                        for (value, label) in stats {
                            StatCard { value, label }
                        }
                    }
                }

                // 2. 筛选与操作区
                section {
                    id: "channels-sec-filter",
                    class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
                    div { class: "flex items-center justify-between gap-3",
                        div { class: "flex items-center gap-2",
                            h2 { class: "text-sm font-medium text-zinc-300", "{SEC_FILTER}" }
                            span { class: "text-xs text-zinc-500", "按状态或关键词筛选" }
                        }
                        div { class: "flex items-center gap-2",
                            button {
                                class: "rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-xs font-medium text-zinc-300 transition-colors hover:border-zinc-500 hover:text-white",
                                "data-testid": "refresh-channels",
                                onclick: move |_| reload.set(reload() + 1),
                                "刷新"
                            }
                            button {
                                class: "shrink-0 rounded-xl bg-white px-4 py-2 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                                onclick: open_new,
                                "✚ 新建渠道"
                            }
                        }
                    }

                    input {
                        class: "w-full rounded-xl border border-zinc-700/80 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 outline-none transition focus:border-zinc-500",
                        r#type: "text",
                        placeholder: "搜索渠道名称、类型、分组或 API 目标地址...",
                        value: "{search}",
                        oninput: move |e| search.set(e.value()),
                    }
                    div { class: "flex flex-wrap gap-2",
                        SegmentedCapsule {
                            items: filter_options,
                            active: filter_tier(),
                            on_select: move |i: usize| filter_tier.set(i),
                        }
                    }
                }

                // 3. 卡片网格区
                section { id: "channels-sec-list", class: "scroll-mt-8 space-y-4",
                    div { class: "flex items-center justify-between",
                        h2 { class: "text-lg font-medium text-zinc-100", "{SEC_LIST}" }
                        span { class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400",
                            if loading() { "加载中…" } else { "{filtered.len()} 个渠道" }
                        }
                    }

                    if let Some(e) = err() {
                        div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 py-10 text-center",
                            p { class: "text-sm text-red-300", "加载渠道失败" }
                            p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                            button {
                                class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                                onclick: move |_| reload.set(reload() + 1),
                                "重试"
                            }
                        }
                    } else if loading() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "正在加载渠道…" }
                        }
                    } else if filtered.is_empty() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "没有匹配的渠道" }
                        }
                    } else {
                        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                            "data-testid": "channels-list",
                            for c in filtered {
                                {
                                    let edit_key = c.key.clone();
                                    let toggle_key = c.key.clone();
                                    let delete_key = c.key.clone();
                                    let target = if c.status == 1 { 0 } else { 1 };
                                    rsx! {
                                        ChannelCard {
                                            key: "{c.key}",
                                            channel: c,
                                            on_edit: move |_| open_edit(edit_key.clone()),
                                            on_toggle: move |_| write_toggle(toggle_key.clone(), WriteOp::Toggle(target)),
                                            on_delete: move |_| write_delete(delete_key.clone(), WriteOp::Delete),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 新建 / 编辑弹窗
            if matches!(modal_state(), ChannelModalState::New | ChannelModalState::Edit(_)) {
                ChannelFormModal {
                    editing: matches!(modal_state(), ChannelModalState::Edit(_)),
                    channel_key: match modal_state() {
                        ChannelModalState::Edit(k) => Some(k),
                        _ => None,
                    },
                    name: f_name,
                    ctype: f_ctype,
                    url: f_url,
                    keys: f_keys,
                    group: f_group,
                    remark: f_remark,
                    on_cancel: move |_| modal_state.set(ChannelModalState::Closed),
                    on_submit: close_and_reload,
                }
            }
    }
}

/// 单个渠道卡片 (对齐 GroupCard / UserCard 规范)
#[component]
fn ChannelCard(
    channel: ChannelDto,
    on_edit: EventHandler<()>,
    on_toggle: EventHandler<()>,
    on_delete: EventHandler<()>,
) -> Element {
    let initial = channel
        .name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();

    let is_enabled = channel.status == 1;

    let (status_text, status_tone) = if is_enabled {
        (
            "启用中",
            "border-emerald-500/30 bg-emerald-500/20 text-emerald-400",
        )
    } else {
        ("已停用", "border-zinc-700 bg-zinc-800/80 text-zinc-400")
    };

    rsx! {
        div { class: "group flex flex-col justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            "data-testid": "channel-card",
            div { class: "space-y-3",
                // 头部
                div { class: "flex items-start gap-3",
                    div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-zinc-700 bg-zinc-800 text-sm font-semibold text-zinc-200 group-hover:border-zinc-500 transition-colors",
                        "{initial}"
                    }
                    div { class: "min-w-0 flex-1",
                        div { class: "flex items-center justify-between gap-2",
                            h3 { class: "truncate text-sm font-medium text-zinc-100", "{channel.name}" }
                            span { class: "shrink-0 rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] font-mono text-zinc-400 border border-zinc-700/60",
                                "#{channel.key}"
                            }
                        }
                        p { class: "mt-0.5 truncate text-[11px] text-zinc-400", "{channel.channel_type} · {channel.group_name}" }
                    }
                }

                // 徽标行
                div { class: "flex flex-wrap gap-1.5",
                    Badge { text: status_text.to_string(), tone: status_tone }
                    Badge { text: channel.group_name.clone(), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-300" }
                    Badge { text: format!("{} 密钥", channel.key_count), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-500" }
                }

                // 指标详情行
                div { class: "space-y-1.5 text-xs pt-1",
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "接口地址" }
                        span { class: "truncate font-mono text-zinc-300 max-w-[140px]", title: "{channel.base_url}", "{channel.base_url}" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "权重" }
                        span { class: "font-medium text-zinc-200", "{channel.weight}" }
                    }
                    if !channel.remark.is_empty() {
                        div { class: "flex justify-between gap-2",
                            span { class: "shrink-0 text-zinc-400", "备注" }
                            span { class: "truncate font-medium text-zinc-200", "{channel.remark}" }
                        }
                    }
                }
            }

            // 底部操作区 (标准三键布局: [编辑] [启用/停用] [删除])
            div { class: "mt-4 flex gap-1.5 border-t border-zinc-800 pt-3",
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white",
                    onclick: move |_| on_edit.call(()),
                    "编辑"
                }
                button {
                    class: if is_enabled {
                        "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-amber-400 transition-colors hover:bg-zinc-700 hover:text-amber-300"
                    } else {
                        "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-emerald-400 transition-colors hover:bg-zinc-700 hover:text-emerald-300"
                    },
                    onclick: move |_| on_toggle.call(()),
                    if is_enabled { "停用" } else { "启用" }
                }
                button {
                    class: "w-7 rounded-lg border border-zinc-800 bg-zinc-800/40 py-1.5 text-xs text-zinc-500 hover:text-red-400 hover:border-red-900/60 transition-colors flex items-center justify-center",
                    title: "删除渠道",
                    onclick: move |_| on_delete.call(()),
                    "✕"
                }
            }
        }
    }
}

/// 渠道编辑/新建综合弹窗 (含类型、名称、URL、Key、分组、备注;后端暂不支持模型调度候补)
#[component]
fn ChannelFormModal(
    editing: bool,
    channel_key: Option<String>,
    name: Signal<String>,
    ctype: Signal<String>,
    url: Signal<String>,
    keys: Signal<String>,
    group: Signal<String>,
    remark: Signal<String>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let title = if editing {
        "编辑渠道"
    } else {
        "新建渠道"
    };
    let submit_label = if editing {
        "保存修改"
    } else {
        "创建渠道"
    };

    let submitting = use_signal(|| false);

    // 工厂式复制,避免把原 signal 移动出闭包(供 rsx 中 submitting() 继续读取)
    let submitting2 = submitting;
    let on_submit2 = on_submit;
    let channel_key2 = channel_key.clone();
    let do_submit = move |_| {
        let key = channel_key2.clone();
        let n = name.peek().trim().to_string();
        if n.is_empty() {
            return;
        }
        let ct = ctype.peek().clone();
        let u = url.peek().trim().to_string();
        let k: Vec<String> = keys
            .peek()
            .split('\n')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let g = group.peek().clone();
        let rm = remark.peek().clone();
        let (mut sub, cb) = (submitting2, on_submit2);
        spawn(async move {
            sub.set(true);
            let client = ApiClient::shared().clone();
            let req = ChannelUpsertRequest {
                name: n,
                channel_type: ct,
                base_url: u,
                keys: k,
                models: json!([]),
                group_name: g,
                priority: 0,
                weight: 0,
                test_model: None,
                remark: rm,
            };
            let res = match key {
                Some(kk) => update_channel_api(&client, &kk, &req).await,
                None => create_channel_api(&client, &req).await,
            };
            let _ = res;
            sub.set(false);
            cb.call(());
        });
    };

    rsx! {
        Modal { title: title.to_string(), on_close: move |_| on_cancel.call(()),
            div { class: "space-y-4 max-h-[70vh] overflow-y-auto pr-1",
                div { class: "grid grid-cols-2 gap-3",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "渠道类型" }
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
                        label { class: "mb-1.5 block text-xs text-zinc-400", "绑定分组" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3 py-2 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                            placeholder: "default",
                            value: "{group}",
                            oninput: move |e| group.set(e.value()),
                        }
                    }
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "渠道名称" }
                    input {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                        placeholder: "例如: OpenAI 官方, Azure East",
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "Base URL (代理或官方地址)" }
                    input {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                        placeholder: "https://api.openai.com/v1",
                        value: "{url}",
                        oninput: move |e| url.set(e.value()),
                    }
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "API Key (多 Key 可换行)" }
                    textarea {
                        class: "w-full h-20 rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none resize-none",
                        placeholder: "sk-...",
                        value: "{keys}",
                        oninput: move |e| keys.set(e.value()),
                    }
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "备注 (可选)" }
                    textarea {
                        class: "w-full h-16 rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none resize-none",
                        placeholder: "渠道用途说明",
                        value: "{remark}",
                        oninput: move |e| remark.set(e.value()),
                    }
                }
            }

            div { class: "mt-6 flex gap-3",
                button {
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                    onclick: move |_| on_cancel.call(()),
                    "取消"
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
