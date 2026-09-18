//! 渠道管理页:卡片式网格,对齐 GroupsPage / UsersPanel 规范。
//! 数据来自真实后端:挂载时 `use_effect` 拉 `list_channels_api`,写入本地
//! `channels` signal;启用/停用走 `set_channel_status_api`,删除走
//! `delete_channel_api`,新建/编辑走 `create_channel_api` / `update_channel_api`。
//! 编辑保存用最小 diff 体(`UpdateChannelBody`):密钥框留空 = 不触碰现有密钥,
//! `models` 仅在用户动过「拉取模型」面板时携带,priority/weight 不随请求发出、
//! 由后端 COALESCE 保持。
//! 拓扑测速、批量分组、快速导入是 mock 期的纯前端特性,已移除(后端暂无对应接口)。
//!
//! 本文件只保留状态、拉取与写回逻辑(submit / open_edit / toggle / delete)
//! 以及组件组合;单卡渲染见 `tab-page_channels_card`,弹窗见
//! `modal`,共享类型与纯函数见 `shared`。

use dioxus::prelude::*;

use ui::ChannelCard as PrototypeChannelCard;
use ui::SegmentedCapsule;

use client::ApiClient;
use contract::api::admin::{ChannelDto, GroupDto};

use crate::api::{
    delete_channel_api, get_channel_api, list_channels_api, list_groups_api,
    set_channel_status_api,
};
use crate::tab_page_groups::StatCard;

use super::card::ChannelCard;
use super::modal::ChannelFormModal;
use super::shared::{ChannelModalState, WriteOp, filter_channels};

const SEC_STATS: &str = "渠道概览";
const SEC_FILTER: &str = "筛选与操作";
const SEC_LIST: &str = "渠道列表";

#[component]
pub fn ChannelsPage() -> Element {
    // 真实数据 + 加载/错误态(本地 signal,不触碰 EntityStore)
    let mut channels = use_signal(Vec::<ChannelDto>::new);
    let mut loading = use_signal(|| true);
    // 后台刷新态:与 `loading` 分离,渲染层不清空列表,只在计数徽标上提示。
    // 合并成一个 loading 会让写操作后的重拉把列表换成占位卡 → 视觉闪烁。
    let mut refreshing = use_signal(|| false);
    // 弹窗「绑定分组」候选（真实分组列表,失败留空 → 弹窗内只读回退）。
    let mut group_options = use_signal(Vec::<GroupDto>::new);
    let mut group_err = use_signal(|| None::<String>);
    // 弹窗「当前密钥（掩码）」与「拉取模型」面板状态。掩码值只读展示,
    // 永不进入提交请求体;model_pool 为 (模型 id, 勾选) 候选池。
    let mut existing_keys = use_signal(Vec::<String>::new);
    let mut model_pool = use_signal(Vec::<(String, bool)>::new);
    let mut models_touched = use_signal(|| false);
    let mut fetching_models = use_signal(|| false);
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
    let mut f_group = use_signal(Vec::<String>::new);
    let mut f_remark = use_signal(String::new);
    // 测速模型：弹窗无编辑控件，但后端 test_model 列是 SQL 直绑（无 COALESCE），
    // 编辑保存必须原样回传现值，缺席即被清成 NULL。编辑打开时从列表行带入。
    let mut f_test_model = use_signal(|| None::<String>);

    // 挂载即拉取真实列表;reload 变化时重拉。
    // 首屏(列表为空)走 `loading` → 渲染占位卡;已有数据的重拉走 `refreshing`
    // → 旧列表留在屏上,避免写操作后整块消失再出现的闪烁。
    // `peek()` 读列表长度:普通 `channels()` 会让本 effect 订阅自己写入的信号 → 重拉死循环。
    use_effect(move || {
        let _ = reload();
        let first_load = channels.peek().is_empty();
        if first_load {
            loading.set(true);
        } else {
            refreshing.set(true);
        }
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            match list_channels_api(&client).await {
                Ok(list) => channels.set(list),
                // 后台刷新失败也走 err:错误态优先于陈旧列表,避免用户对着过期数据操作
                Err(e) => err.set(Some(e.to_string())),
            }
            loading.set(false);
            refreshing.set(false);
        });
    });

    // 分组候选:弹窗「绑定分组」多选用。与列表并行拉取,失败只记 err 文案,
    // 不阻断弹窗——弹窗在候选为空时回退为只读展示当前分组值(见 ChannelFormModal)。
    use_effect(move || {
        let _ = reload();
        spawn(async move {
            let client = ApiClient::shared().clone();
            match list_groups_api(&client).await {
                Ok(list) => {
                    group_options.set(list);
                    group_err.set(None);
                }
                Err(e) => group_err.set(Some(e.to_string())),
            }
        });
    });

    let list = channels();
    let total = list.len();
    let enabled_count = list.iter().filter(|c| c.status == 1).count();
    let disabled_count = list.iter().filter(|c| c.status != 1).count();
    let total_keys: i64 = list.iter().map(|c| c.key_count).sum();
    let group_set: std::collections::BTreeSet<String> =
        list.iter().flat_map(|c| c.groups.iter().cloned()).collect();

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

    // 筛选纯函数:抽取为模块级 `filter_channels`,便于纯函数单测
    let filtered = filter_channels(&list, &search(), filter_tier());

    let open_new = move |_| {
        f_name.set(String::new());
        f_ctype.set("openai".to_string());
        f_url.set("https://api.openai.com/v1".to_string());
        f_keys.set(String::new());
        f_group.set(vec!["default".to_string()]);
        existing_keys.set(Vec::new());
        model_pool.set(Vec::new());
        models_touched.set(false);
        fetching_models.set(false);
        f_remark.set(String::new());
        f_test_model.set(None);
        modal_state.set(ChannelModalState::New);
    };

    let mut open_edit = move |key: String| {
        if let Some(c) = channels().iter().find(|c| c.key == key) {
            f_name.set(c.name.clone());
            f_ctype.set(c.channel_type.clone());
            f_url.set(c.base_url.clone());
            // 密钥编辑框留空：不回显掩码，最小 diff 语义是「留空 = 不改动现有密钥」。
            // 当前密钥的掩码值单独只读展示（单查接口带回），让维护者知道配置了什么。
            f_keys.set(String::new());
            f_group.set(c.groups.clone());
            f_remark.set(c.remark.clone());
            f_test_model.set(c.test_model.clone());
            // 模型候选池预填渠道现有 models（字符串或 {alias} 对象，取别名），
            // 全部勾选 = 现状；未动面板则 models 字段缺席（COALESCE 保持）。
            let existing: Vec<String> = c
                .models
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|m| {
                            m.as_str().map(|s| s.to_string()).or_else(|| {
                                m.get("alias")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            model_pool.set(existing.iter().cloned().map(|id| (id, true)).collect());
            models_touched.set(false);
            fetching_models.set(false);
            existing_keys.set(Vec::new());
            modal_state.set(ChannelModalState::Edit(key.clone()));
            // 掩码密钥与分组候选并行拉取（失败静默：掩码区隐藏/候选区只读回退）
            spawn(async move {
                if let Ok(dto) = get_channel_api(&ApiClient::shared().clone(), &key).await
                    && let Some(ks) = dto.keys
                {
                    existing_keys.set(ks);
                }
            });
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
                                "data-testid": "new-channel",
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
                        if let Some(channel) = filtered.first().cloned() {
                            {
                                rsx! {
                                    div {
                                        class: "mb-4 grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                                        role: "region",
                                        "aria-label": "新卡示例",
                                        "data-testid": "channel-card-prototype",
                                        PrototypeChannelCard {
                                            channel,
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                            "data-testid": "channels-list",
                            for c in filtered {
                                {
                                    let edit_key = c.key.clone();
                                    let toggle_key = c.key.clone();
                                    let delete_key = c.key.clone();
                                    // 后端 status 语义: 1=启用 2=停用 (channels status 校验 [1,2])
                                    let target = if c.status == 1 { 2 } else { 1 };
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
                    group_options: group_options,
                    group_err: group_err,
                    existing_keys: existing_keys,
                    model_pool: model_pool,
                    models_touched: models_touched,
                    fetching_models: fetching_models,
                    remark: f_remark,
                    test_model: f_test_model,
                    on_cancel: move |_| modal_state.set(ChannelModalState::Closed),
                    on_submit: close_and_reload,
                }
            }
    }
}
