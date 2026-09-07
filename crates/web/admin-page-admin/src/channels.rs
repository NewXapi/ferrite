//! 渠道管理页:卡片式网格,对齐 GroupsPage / UsersPanel 规范。
//! 包含:顶部渠道概览、综合筛选与批量操作、卡片网格、编辑与模型调度抽屉/弹窗、快速导入弹窗。

use dioxus::prelude::*;
use ui::SegmentedCapsule;

use crate::groups::{Badge, Modal, StatCard};
use crate::pages::parse_url_key;
use crate::state::{CHANNEL_TYPES, ChannelRow, EntityStore};

/// 弹窗状态
#[derive(Clone, PartialEq)]
enum ChannelModalState {
    Closed,
    New,
    Edit(usize),
    Import,
    BatchGroup,
}

const SEC_STATS: &str = "渠道概览";
const SEC_FILTER: &str = "筛选与操作";
const SEC_LIST: &str = "渠道列表";

#[component]
pub fn ChannelsPage() -> Element {
    let store = use_context::<EntityStore>();
    let channels = store.channels;
    let groups = store.groups;

    let mut search = use_signal(String::new);
    let mut filter_tier = use_signal(|| 0usize);
    let mut modal_state = use_signal(|| ChannelModalState::Closed);
    let mut testing_idx = use_signal(|| None::<usize>);
    let mut test_counter = use_signal(|| 0u32);

    // 编辑/新建表单状态
    let mut f_name = use_signal(String::new);
    let mut f_ctype = use_signal(|| "openai".to_string());
    let mut f_url = use_signal(String::new);
    let mut f_keys = use_signal(String::new);
    let mut f_group = use_signal(|| "default".to_string());

    // 批量改分组目标
    let mut batch_group_target = use_signal(|| "default".to_string());

    // 导入表单状态
    let mut import_raw = use_signal(String::new);
    let mut import_url = use_signal(String::new);
    let mut import_key = use_signal(String::new);
    let mut import_name = use_signal(String::new);

    let channel_list = channels.read().clone();
    let total = channel_list.len();
    let enabled_count = channel_list.iter().filter(|c| c.status == 1).count();
    let disabled_count = channel_list.iter().filter(|c| c.status != 1).count();

    // 计算有测试延迟的渠道平均值
    let measured_latencies: Vec<i32> = channel_list.iter().filter_map(|c| c.latency_ms).collect();
    let avg_latency = if !measured_latencies.is_empty() {
        let sum: i32 = measured_latencies.iter().sum();
        format!("{}ms", sum / (measured_latencies.len() as i32))
    } else {
        "—".to_string()
    };

    let total_dispatched_models: usize = channel_list.iter().map(|c| c.dispatch.len()).sum();

    let stats: [(String, &str); 5] = [
        (total.to_string(), "总渠道数"),
        (enabled_count.to_string(), "正常启用"),
        (disabled_count.to_string(), "停用/异常"),
        (avg_latency, "平均响应延迟"),
        (total_dispatched_models.to_string(), "总调度模型数"),
    ];

    let filter_options = vec![
        format!("全部 ({total})"),
        format!("启用中 ({enabled_count})"),
        format!("已停用 ({disabled_count})"),
        "OpenAI 兼容".to_string(),
        "Claude / 其他".to_string(),
    ];

    let filtered_indices: Vec<usize> = {
        let q = search().trim().to_lowercase();
        let tier = filter_tier();
        channel_list
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                if !q.is_empty()
                    && !c.name.to_lowercase().contains(&q)
                    && !c.ctype.to_lowercase().contains(&q)
                    && !c.url.to_lowercase().contains(&q)
                    && !c.group.to_lowercase().contains(&q)
                {
                    return false;
                }
                match tier {
                    1 => c.status == 1,
                    2 => c.status != 1,
                    3 => c.ctype.contains("openai"),
                    4 => !c.ctype.contains("openai"),
                    _ => true,
                }
            })
            .map(|(i, _)| i)
            .collect()
    };

    let open_new = move |_| {
        f_name.set(String::new());
        f_ctype.set("openai".to_string());
        f_url.set("https://api.openai.com/v1".to_string());
        f_keys.set(String::new());
        f_group.set("default".to_string());
        modal_state.set(ChannelModalState::New);
    };

    let open_edit = move |idx: usize| {
        if let Some(c) = channels.read().get(idx) {
            f_name.set(c.name.clone());
            f_ctype.set(c.ctype.clone());
            f_url.set(c.url.clone());
            f_keys.set(c.keys.clone());
            f_group.set(c.group.clone());
            modal_state.set(ChannelModalState::Edit(idx));
        }
    };

    let toggle_status = move |idx: usize| {
        let mut ch = channels;
        if idx < ch.read().len() {
            let cur = ch.read()[idx].status;
            ch.write()[idx].status = if cur == 1 { 0 } else { 1 };
        }
    };

    let test_single = move |idx: usize| {
        testing_idx.set(Some(idx));
        let n = test_counter.peek().wrapping_add(1);
        test_counter.set(n);
        let ms = 120 + (n.wrapping_mul(97) % 380);
        let mut ch = channels;
        spawn(async move {
            gloo_timers::future::TimeoutFuture::new(500).await;
            if idx < ch.read().len() {
                ch.write()[idx].latency_ms = Some(ms.min(i32::MAX as u32) as i32);
            }
            testing_idx.set(None);
        });
    };

    let test_all = move |_| {
        let n = test_counter.peek().wrapping_add(1);
        test_counter.set(n);
        let mut ch = channels;
        let count = ch.read().len();
        spawn(async move {
            for i in 0..count {
                let ms = 110 + ((n.wrapping_add(i as u32)).wrapping_mul(73) % 420);
                ch.write()[i].latency_ms = Some(ms.min(i32::MAX as u32) as i32);
            }
        });
    };

    let delete_channel = move |idx: usize| {
        let mut ch = channels;
        if idx < ch.read().len() {
            ch.write().remove(idx);
        }
    };

    let commit_edit = move |_| {
        let n = f_name.peek().trim().to_string();
        if n.is_empty() {
            return;
        }
        let ct = f_ctype.peek().clone();
        let u = f_url.peek().trim().to_string();
        let k = f_keys.peek().trim().to_string();
        let g = f_group.peek().clone();

        let mut ch = channels;
        match *modal_state.peek() {
            ChannelModalState::New => {
                ch.write().push(ChannelRow {
                    name: n,
                    ctype: ct,
                    url: u,
                    keys: k,
                    status: 1,
                    group: g,
                    latency_ms: None,
                    candidates: vec![
                        ("gpt-4o".to_string(), false),
                        ("gpt-4o-mini".to_string(), false),
                    ],
                    dispatch: vec!["gpt-4o".to_string()],
                });
            }
            ChannelModalState::Edit(idx) => {
                let mut w = ch.write();
                if idx < w.len() {
                    w[idx].name = n;
                    w[idx].ctype = ct;
                    w[idx].url = u;
                    w[idx].keys = k;
                    w[idx].group = g;
                }
            }
            _ => {}
        }
        modal_state.set(ChannelModalState::Closed);
    };

    let commit_batch_group = move |_| {
        let target = batch_group_target.peek().clone();
        let mut ch = channels;
        for c in ch.write().iter_mut() {
            c.group = target.clone();
        }
        modal_state.set(ChannelModalState::Closed);
    };

    let commit_import = move |_| {
        let u = import_url.peek().trim().to_string();
        let k = import_key.peek().trim().to_string();
        if u.is_empty() || k.is_empty() {
            return;
        }
        let n = if !import_name.peek().trim().is_empty() {
            import_name.peek().trim().to_string()
        } else {
            "导入渠道".to_string()
        };
        let mut ch = channels;
        ch.write().push(ChannelRow {
            name: n,
            ctype: "openai-compat".to_string(),
            url: u,
            keys: k,
            status: 1,
            group: "default".to_string(),
            latency_ms: None,
            candidates: vec![("gpt-4o".to_string(), false)],
            dispatch: vec!["gpt-4o".to_string()],
        });
        import_raw.set(String::new());
        import_url.set(String::new());
        import_key.set(String::new());
        import_name.set(String::new());
        modal_state.set(ChannelModalState::Closed);
    };

    rsx! {
        div { class: "flex flex-col gap-6",
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
                            span { class: "text-xs text-zinc-500", "按状态、类型或关键词筛选" }
                        }
                        div { class: "flex items-center gap-2",
                            button {
                                class: "rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-xs font-medium text-zinc-300 transition-colors hover:border-zinc-500 hover:text-white",
                                onclick: test_all,
                                "⚡ 一键测速"
                            }
                            button {
                                class: "rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-xs font-medium text-zinc-300 transition-colors hover:border-zinc-500 hover:text-white",
                                onclick: move |_| modal_state.set(ChannelModalState::BatchGroup),
                                "批量分组"
                            }
                            button {
                                class: "rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-xs font-medium text-zinc-300 transition-colors hover:border-zinc-500 hover:text-white",
                                onclick: move |_| modal_state.set(ChannelModalState::Import),
                                "📋 导入"
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
                            "{filtered_indices.len()} 个渠道"
                        }
                    }

                    if filtered_indices.is_empty() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "没有匹配的渠道" }
                        }
                    } else {
                        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                            for idx in filtered_indices {
                                {
                                    let c = channels.read()[idx].clone();
                                    let is_testing = testing_idx() == Some(idx);
                                    rsx! {
                                        ChannelCard {
                                            key: "{c.name}_{idx}",
                                            channel: c,
                                            index: idx,
                                            is_testing: is_testing,
                                            on_edit: open_edit,
                                            on_toggle: toggle_status,
                                            on_test: test_single,
                                            on_delete: delete_channel,
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
                    name: f_name,
                    ctype: f_ctype,
                    url: f_url,
                    keys: f_keys,
                    group: f_group,
                    channel_idx: match modal_state() {
                        ChannelModalState::Edit(idx) => Some(idx),
                        _ => None,
                    },
                    on_cancel: move |_| modal_state.set(ChannelModalState::Closed),
                    on_submit: commit_edit,
                }
            }

            // 导入弹窗
            if modal_state() == ChannelModalState::Import {
                ChannelImportModal {
                    raw: import_raw,
                    url: import_url,
                    api_key: import_key,
                    name: import_name,
                    on_cancel: move |_| modal_state.set(ChannelModalState::Closed),
                    on_submit: commit_import,
                }
            }

            // 批量改分组弹窗
            if modal_state() == ChannelModalState::BatchGroup {
                Modal {
                    title: "批量绑定分组".to_string(),
                    on_close: move |_| modal_state.set(ChannelModalState::Closed),
                    div { class: "space-y-4",
                        p { class: "text-xs text-zinc-400 leading-relaxed",
                            "将全部渠道切换到指定的目标分组中，生效后服务拓扑将同步更新。"
                        }
                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "目标分组" }
                            select {
                                class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                                value: "{batch_group_target}",
                                onchange: move |e| batch_group_target.set(e.value()),
                                for g in groups.read().iter() {
                                    option { value: "{g.name}", "{g.name} ({g.display})" }
                                }
                            }
                        }
                        div { class: "mt-6 flex gap-3",
                            button {
                                class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                                onclick: move |_| modal_state.set(ChannelModalState::Closed),
                                "取消"
                            }
                            button {
                                class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200",
                                onclick: commit_batch_group,
                                "应用到全部渠道"
                            }
                        }
                    }
                }
            }
    }
}

/// 单个渠道卡片 (对齐 GroupCard / UserCard 规范)
#[component]
fn ChannelCard(
    channel: ChannelRow,
    index: usize,
    is_testing: bool,
    on_edit: EventHandler<usize>,
    on_toggle: EventHandler<usize>,
    on_test: EventHandler<usize>,
    on_delete: EventHandler<usize>,
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

    let (latency_text, latency_tone, latency_bar_tone, latency_pct) = match channel.latency_ms {
        Some(ms) if ms < 250 => (
            format!("{ms}ms"),
            "border-emerald-500/30 bg-emerald-500/20 text-emerald-400",
            "bg-emerald-500",
            ((ms as f64 / 800.0) * 100.0).clamp(15.0, 100.0) as u32,
        ),
        Some(ms) if ms < 600 => (
            format!("{ms}ms"),
            "border-amber-500/30 bg-amber-500/20 text-amber-400",
            "bg-amber-500",
            ((ms as f64 / 800.0) * 100.0).clamp(15.0, 100.0) as u32,
        ),
        Some(ms) => (
            format!("{ms}ms"),
            "border-red-500/30 bg-red-500/20 text-red-400",
            "bg-red-500",
            95,
        ),
        None => (
            "未测速".to_string(),
            "border-zinc-700 bg-zinc-800/80 text-zinc-500",
            "bg-zinc-700",
            20,
        ),
    };

    // 掩码 API Key:只展示首尾
    let masked_key = if channel.keys.len() > 8 {
        let prefix = &channel.keys[..4.min(channel.keys.len())];
        let suffix = &channel.keys[channel.keys.len().saturating_sub(4)..];
        format!("{prefix}****{suffix}")
    } else if !channel.keys.is_empty() {
        "sk-****".to_string()
    } else {
        "未配置密钥".to_string()
    };

    let dispatch_count = channel.dispatch.len();

    rsx! {
        div { class: "group flex flex-col justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
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
                                "#{index + 1}"
                            }
                        }
                        p { class: "mt-0.5 truncate text-[11px] text-zinc-400", "{channel.ctype} · {channel.group}" }
                    }
                }

                // 徽标行
                div { class: "flex flex-wrap gap-1.5",
                    Badge { text: status_text.to_string(), tone: status_tone }
                    Badge { text: channel.group.clone(), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-300" }
                    Badge { text: latency_text, tone: latency_tone }
                }

                // 延迟进度条
                div { class: "space-y-1.5",
                    div { class: "flex justify-between gap-2 text-[11px]",
                        span { class: "text-zinc-400", "延迟响应" }
                        if let Some(ms) = channel.latency_ms {
                            span { class: "whitespace-nowrap font-medium text-zinc-200 font-mono", "{ms} ms" }
                        } else {
                            span { class: "whitespace-nowrap font-medium text-zinc-500", "未测试" }
                        }
                    }
                    div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                        div { class: "h-full rounded-full {latency_bar_tone} transition-all duration-300", style: "width: {latency_pct}%" }
                    }
                }

                // 指标详情行
                div { class: "space-y-1.5 text-xs pt-1",
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "接口地址" }
                        span { class: "truncate font-mono text-zinc-300 max-w-[140px]", title: "{channel.url}", "{channel.url}" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "密钥配置" }
                        span { class: "font-mono text-zinc-400", "{masked_key}" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "调度模型" }
                        span { class: "font-medium text-zinc-200", "{dispatch_count} 个已进拓扑" }
                    }
                }
            }

            // 底部操作区 (标准三键布局: [编辑] [测速] [启用/停用])
            div { class: "mt-4 flex gap-1.5 border-t border-zinc-800 pt-3",
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white",
                    onclick: move |_| on_edit.call(index),
                    "编辑"
                }
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-emerald-400 transition-colors hover:bg-zinc-700 hover:text-emerald-300 disabled:opacity-40",
                    disabled: is_testing,
                    onclick: move |_| on_test.call(index),
                    if is_testing { "测速中" } else { "测速" }
                }
                button {
                    class: if is_enabled {
                        "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-amber-400 transition-colors hover:bg-zinc-700 hover:text-amber-300"
                    } else {
                        "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-zinc-400 transition-colors hover:bg-zinc-700 hover:text-zinc-200"
                    },
                    onclick: move |_| on_toggle.call(index),
                    if is_enabled { "停用" } else { "启用" }
                }
                button {
                    class: "w-7 rounded-lg border border-zinc-800 bg-zinc-800/40 py-1.5 text-xs text-zinc-500 hover:text-red-400 hover:border-red-900/60 transition-colors flex items-center justify-center",
                    title: "删除渠道",
                    onclick: move |_| on_delete.call(index),
                    "✕"
                }
            }
        }
    }
}

/// 渠道编辑/新建综合弹窗 (含类型、名称、URL、Key、分组、模型候补与调度)
#[component]
fn ChannelFormModal(
    editing: bool,
    name: Signal<String>,
    ctype: Signal<String>,
    url: Signal<String>,
    keys: Signal<String>,
    group: Signal<String>,
    channel_idx: Option<usize>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let store = use_context::<EntityStore>();
    let mut channels = store.channels;
    let groups = store.groups;

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

    let pull_models = move |_| {
        if let Some(idx) = channel_idx {
            let pool = [
                "gpt-4o",
                "gpt-4o-mini",
                "gpt-5",
                "o3",
                "o3-mini",
                "claude-3-5-sonnet",
            ];
            let mut w = channels.write();
            let c = &mut w[idx];
            let have: Vec<String> = c
                .candidates
                .iter()
                .map(|(n, _)| n.clone())
                .chain(c.dispatch.iter().cloned())
                .collect();
            for m in pool {
                if !have.iter().any(|x| x == m) {
                    c.candidates.push((m.to_string(), false));
                }
            }
        }
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
                        select {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3 py-2 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                            value: "{group}",
                            onchange: move |e| group.set(e.value()),
                            for g in groups.read().iter() {
                                option { value: "{g.name}", "{g.name} ({g.display})" }
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

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "API Key (多 Key 可换行)" }
                    textarea {
                        class: "w-full h-20 rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none resize-none",
                        placeholder: "sk-...",
                        value: "{keys}",
                        oninput: move |e| keys.set(e.value()),
                    }
                }

                // 模型调度管理 (仅编辑状态呈现)
                if let Some(idx) = channel_idx {
                    div { class: "rounded-xl border border-zinc-800 bg-zinc-950 p-3 space-y-3",
                        div { class: "flex items-center justify-between",
                            div {
                                p { class: "text-xs font-medium text-zinc-200", "模型候补池与调度" }
                                p { class: "text-[11px] text-zinc-500", "拉取上游模型并加入拓扑调度" }
                            }
                            button {
                                class: "rounded-lg border border-zinc-700 bg-zinc-900 px-2.5 py-1 text-xs text-zinc-300 hover:border-zinc-500 transition-colors",
                                onclick: pull_models,
                                "拉取模型"
                            }
                        }

                        // 候补列表
                        if !channels.read()[idx].candidates.is_empty() {
                            div { class: "space-y-1",
                                p { class: "text-[11px] text-zinc-400", "候补池 (勾选后加入调度):" }
                                div { class: "flex flex-wrap gap-1.5",
                                    for (j, (m, on)) in channels.read()[idx].candidates.iter().enumerate() {
                                        {
                                            let label = m.clone();
                                            let checked = *on;
                                            rsx! {
                                                button {
                                                    class: if checked {
                                                        "rounded-md border border-zinc-100 bg-zinc-100 px-2 py-0.5 text-xs text-zinc-900 font-mono transition-colors"
                                                    } else {
                                                        "rounded-md border border-zinc-800 bg-zinc-900 px-2 py-0.5 text-xs text-zinc-400 font-mono hover:border-zinc-600 transition-colors"
                                                    },
                                                    onclick: move |_| {
                                                        let mut w = channels.write();
                                                        let cur = w[idx].candidates[j].1;
                                                        w[idx].candidates[j].1 = !cur;
                                                    },
                                                    "{label}"
                                                }
                                            }
                                        }
                                    }
                                }
                                div { class: "pt-1 flex gap-2",
                                    button {
                                        class: "rounded-md bg-zinc-100 px-2.5 py-1 text-[11px] font-medium text-zinc-900 hover:bg-zinc-300",
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
                                        "将勾选加入调度"
                                    }
                                }
                            }
                        }

                        // 已调度模型标签
                        div { class: "space-y-1 pt-1 border-t border-zinc-800/80",
                            p { class: "text-[11px] text-zinc-400", "已进调度拓扑的模型:" }
                            if channels.read()[idx].dispatch.is_empty() {
                                p { class: "text-[11px] text-zinc-600", "暂无模型，请从候补池添加" }
                            } else {
                                div { class: "flex flex-wrap gap-1.5",
                                    for (j, m) in channels.read()[idx].dispatch.iter().enumerate() {
                                        {
                                            let label = m.clone();
                                            rsx! {
                                                div { class: "flex items-center gap-1 rounded-md border border-zinc-700 bg-zinc-900 px-2 py-0.5 text-xs font-mono text-zinc-200",
                                                    span { "{label}" }
                                                    button {
                                                        class: "text-zinc-500 hover:text-red-400 ml-1",
                                                        onclick: move |_| {
                                                            let mut w = channels.write();
                                                            let rem = w[idx].dispatch.remove(j);
                                                            w[idx].candidates.push((rem, false));
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

            div { class: "mt-6 flex gap-3",
                button {
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                    onclick: move |_| on_cancel.call(()),
                    "取消"
                }
                button {
                    class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200",
                    onclick: move |_| on_submit.call(()),
                    "{submit_label}"
                }
            }
        }
    }
}

/// 渠道快速解析与导入弹窗
#[component]
fn ChannelImportModal(
    raw: Signal<String>,
    url: Signal<String>,
    api_key: Signal<String>,
    name: Signal<String>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let mut parsed_state = use_signal(|| None::<bool>);

    rsx! {
        Modal { title: "快速导入渠道".to_string(), on_close: move |_| on_cancel.call(()),
            div { class: "space-y-4",
                p { class: "text-xs text-zinc-400 leading-relaxed",
                    "在下方输入框中粘贴包含 Base URL 和 API Key 的文本，系统将自动识别并解析提取。"
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "快速粘贴 (支持 URL Key 或 Key=Value)" }
                    textarea {
                        class: "w-full h-24 rounded-xl border border-dashed border-zinc-700 bg-zinc-950 px-4 py-2.5 text-xs text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none resize-none",
                        placeholder: "例如:\nhttps://api.openai.com/v1\nsk-proj-123456789...",
                        value: "{raw}",
                        oninput: move |e| {
                            let text = e.value();
                            raw.set(text.clone());
                            if let Some((u, k)) = parse_url_key(&text) {
                                url.set(u);
                                api_key.set(k);
                                parsed_state.set(Some(true));
                            } else {
                                parsed_state.set(Some(false));
                            }
                        },
                    }
                }

                if let Some(true) = parsed_state() {
                    div { class: "rounded-xl border border-emerald-800/40 bg-emerald-950/30 p-3 text-xs space-y-1 text-emerald-300",
                        p { "✓ 成功识别并解析目标参数" }
                    }
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "渠道名称 (可选)" }
                    input {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                        placeholder: "OpenAI 代理",
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "解析得到的 Base URL" }
                    input {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                        value: "{url}",
                        oninput: move |e| url.set(e.value()),
                    }
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "解析得到的 API Key" }
                    input {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                        value: "{api_key}",
                        oninput: move |e| api_key.set(e.value()),
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
                    disabled: url().trim().is_empty() || api_key().trim().is_empty(),
                    onclick: move |_| on_submit.call(()),
                    "完成并导入"
                }
            }
        }
    }
}
