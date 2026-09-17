//! 渠道管理页:卡片式网格,对齐 GroupsPage / UsersPanel 规范。
//! 数据来自真实后端:挂载时 `use_effect` 拉 `list_channels_api`,写入本地
//! `channels` signal;启用/停用走 `set_channel_status_api`,删除走
//! `delete_channel_api`,新建/编辑走 `create_channel_api` / `update_channel_api`。
//! 编辑保存用最小 diff 体(`UpdateChannelBody`):密钥框留空 = 不触碰现有密钥,
//! `models` 仅在用户动过「拉取模型」面板时携带,priority/weight 不随请求发出、
//! 由后端 COALESCE 保持。
//! 拓扑测速、批量分组、快速导入是 mock 期的纯前端特性,已移除(后端暂无对应接口)。

use dioxus::prelude::*;
use serde_json::json;
use ui::ChannelCard as PrototypeChannelCard;
use ui::SegmentedCapsule;

use client::ApiClient;
use contract::api::admin::{ChannelDto, ChannelUpsertRequest, GroupDto};

use crate::api::{
    UpdateChannelBody, create_channel_api, delete_channel_api, fetch_channel_models_api,
    get_channel_api, list_channels_api, list_groups_api, set_channel_status_api,
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

/// 渠道列表筛选纯函数:按关键词(名称/类型/地址/分组,大小写不敏感)+
/// 状态档位(0=全部, 1=启用中, 2=已停用)过滤。
///
/// 抽成模块级 `pub` 纯函数,便于在同层 `tests/` 做无 runtime 的纯函数单测。
pub fn filter_channels(list: &[ChannelDto], query: &str, tier: usize) -> Vec<ChannelDto> {
    let q = query.trim().to_lowercase();
    list.iter()
        .filter(|c| {
            if !q.is_empty()
                && !c.name.to_lowercase().contains(&q)
                && !c.channel_type.to_lowercase().contains(&q)
                && !c.base_url.to_lowercase().contains(&q)
                && !c.groups.iter().any(|g| g.to_lowercase().contains(&q))
            {
                return false;
            }
            match tier {
                1 => c.status == 1,
                2 => c.status != 1,
                _ => true,
            }
        })
        .cloned()
        .collect()
}

/// 弹窗「绑定分组」输入解析:逗号分隔,trim,去空项(保持输入顺序)。
pub fn parse_group_input(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 弹窗「API Key」输入解析:换行分隔多 Key,trim,去空行(保持输入顺序)。
pub fn parse_keys_input(raw: &str) -> Vec<String> {
    raw.lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

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
                                let edit_key = channel.key.clone();
                                rsx! {
                                    div {
                                        class: "mb-4 grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                                        role: "region",
                                        "aria-label": "新卡示例",
                                        "data-testid": "channel-card-prototype",
                                        PrototypeChannelCard {
                                            channel,
                                            on_edit: move |_| open_edit(edit_key.clone()),
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
                            // UUID 全串不可断:允许收缩并截断,悬停 title 看全值,避免凸出卡片
                            span { class: "min-w-0 max-w-[140px] truncate rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] font-mono text-zinc-400 border border-zinc-700/60",
                                title: "{channel.key}",
                                "#{channel.key}"
                            }
                        }
                        p { class: "mt-0.5 truncate text-[11px] text-zinc-400", "{channel.channel_type} · {channel.groups.join(\", \")}" }
                    }
                }

                // 徽标行
                div { class: "flex flex-wrap gap-1.5",
                    Badge { text: status_text.to_string(), tone: status_tone }
                    Badge { text: channel.groups.join(", "), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-300" }
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
                    "data-testid": "edit-channel",
                    "编辑"
                }
                button {
                    class: if is_enabled {
                        "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-amber-400 transition-colors hover:bg-zinc-700 hover:text-amber-300"
                    } else {
                        "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-emerald-400 transition-colors hover:bg-zinc-700 hover:text-emerald-300"
                    },
                    onclick: move |_| on_toggle.call(()),
                    "data-testid": "toggle-channel",
                    if is_enabled { "停用" } else { "启用" }
                }
                button {
                    class: "w-7 rounded-lg border border-zinc-800 bg-zinc-800/40 py-1.5 text-xs text-zinc-500 hover:text-red-400 hover:border-red-900/60 transition-colors flex items-center justify-center",
                    title: "删除渠道",
                    "data-testid": "delete-channel",
                    onclick: move |_| on_delete.call(()),
                    "✕"
                }
            }
        }
    }
}

/// 渠道编辑/新建综合弹窗 (含类型、名称、URL、Key、分组、备注;后端暂不支持模型调度候补)。
/// 编辑分支发 [`UpdateChannelBody`] 最小 diff 体;新建分支仍用全量
/// [`ChannelUpsertRequest`]（创建语义要求 keys/models 等字段必须给全）。
/// 保存成功走 `on_submit`（关弹窗+重拉列表），失败弹窗保持打开并内嵌展示
/// `channel-save-error`（role=alert）供就地重试。
#[component]
fn ChannelFormModal(
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
    let title = if editing {
        "编辑渠道"
    } else {
        "新建渠道"
    };
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
        "保存修改"
    } else {
        "创建渠道"
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
                Err(e) => serr.set(Some(format!("保存失败:{e}"))),
            }
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
                        label { class: "mb-1.5 block text-xs text-zinc-400", "绑定分组（点选，可多选）" }
                        div { class: "flex min-h-[38px] flex-wrap items-center gap-1.5 rounded-xl border border-zinc-700 bg-zinc-950 px-2 py-1.5",
                            if group_options.read().is_empty() {
                                // 候选拉取失败/为空：只读展示当前已绑分组，不阻断保存
                                span { class: "text-xs text-zinc-500",
                                    if let Some(e) = group_err.read().as_ref() {
                                        "分组列表拉取失败（{e}）；当前绑定: {bound_groups}"
                                    } else if group.read().is_empty() {
                                        "暂无分组"
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

                if editing && !existing_keys.read().is_empty() {
                    // 掩码只读展示：明文永不出后端（单查接口也回掩码）。
                    // 独立于下方 textarea，物理隔离保证掩码串不可能进入提交体。
                    div { class: "space-y-1",
                        span { class: "block text-[11px] text-zinc-500",
                            "当前密钥（掩码，共 {existing_keys.read().len()} 条；明文不出后端）"
                        }
                        for mk in existing_keys.read().iter() {
                            div { class: "rounded-md border border-zinc-800 bg-zinc-900/60 px-2.5 py-1 font-mono text-xs text-zinc-400", "{mk}" }
                        }
                    }
                }
                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "API Key (多 Key 可换行)" }
                    textarea {
                        class: "w-full h-20 rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none resize-none",
                        placeholder: if editing {
                            "留空 = 保持现有密钥；输入明文则整体替换"
                        } else {
                            "sk-..."
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
                                                serr.set(Some(format!("拉取上游模型失败：{e}")));
                                            }
                                        }
                                        flag.set(false);
                                    });
                                },
                                if *fetching_models.read() { "拉取中…" } else { "拉取上游模型" }
                            }
                            span { class: "text-[11px] text-zinc-500",
                                "用该渠道凭据请求上游 /v1/models；勾选项随保存写入（整体替换该列，已含现有模型）"
                            }
                        }
                        div { class: "max-h-40 overflow-y-auto rounded-xl border border-zinc-700 bg-zinc-950 p-2 space-y-1",
                            if model_pool.read().is_empty() {
                                span { class: "text-[11px] text-zinc-600", "暂无候选模型；点「拉取上游模型」获取" }
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
                    label { class: "mb-1.5 block text-xs text-zinc-400", "备注 (可选)" }
                    textarea {
                        class: "w-full h-16 rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none resize-none",
                        placeholder: "渠道用途说明",
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
                    class: "rounded-xl border border-red-500/30 bg-red-950/30 p-4 text-sm text-red-400",
                    "data-testid": "channel-save-error",
                    "{msg}"
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
