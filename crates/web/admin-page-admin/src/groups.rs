//! 分组管理页:卡片式设计,对齐用户管理面板 (UsersPanel) 视觉规范。
//! 数据来自真实后端:挂载时 `use_effect` 拉 `list_groups_api`,写入本地
//! `groups` signal;删除走 `delete_group_api`,新建/编辑走
//! `create_group_api` / `update_group_api`。

use dioxus::prelude::*;
use serde_json::json;
use ui::SegmentedCapsule;

use client::ApiClient;
use contract::api::admin::{GroupDto, GroupUpsertRequest};

use crate::api::{create_group_api, delete_group_api, list_groups_api, update_group_api};

/// 弹窗状态
#[derive(Clone, PartialEq)]
enum ModalState {
    Closed,
    New,
    Edit(String),
}

/// 写操作种类(目前仅删除;工厂保留扩展位)
#[derive(Clone, Copy)]
enum WriteOp {
    Delete,
}

const SEC_STATS: &str = "分组概览";
const SEC_FILTER: &str = "筛选与操作";
const SEC_LIST: &str = "分组列表";

#[component]
pub fn GroupsPage() -> Element {
    // 真实数据 + 加载/错误态(本地 signal,不触碰 EntityStore)
    let mut groups = use_signal(Vec::<GroupDto>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    // 写操作进行中 / 成功提示
    let busy = use_signal(|| false);
    let notice = use_signal(|| None::<String>);
    // reload 计数:触发一次即重拉列表(写操作后刷新)
    let mut reload = use_signal(|| 0u32);

    let mut search = use_signal(String::new);
    let mut filter_tier = use_signal(|| 0usize);
    let mut modal_state = use_signal(|| ModalState::Closed);

    // 表单状态
    let mut f_name = use_signal(String::new);
    let mut f_ratio = use_signal(|| "1.0".to_string());
    let mut f_remark = use_signal(String::new);

    // 挂载即拉取真实列表;reload 变化时重拉
    use_effect(move || {
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            match list_groups_api(&client).await {
                Ok(list) => {
                    groups.set(list);
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    let list = groups();
    let total = list.len();
    let enabled_count = list.iter().filter(|g| g.status == 1).count();
    let disabled_count = list.iter().filter(|g| g.status != 1).count();
    let avg_ratio = if total > 0 {
        list.iter().map(|g| g.ratio).sum::<f64>() / (total as f64)
    } else {
        1.0
    };
    let custom_count = list
        .iter()
        .filter(|g| (g.ratio - 1.0).abs() > 0.001)
        .count();

    let stats: [(String, &str); 5] = [
        (total.to_string(), "总分组数"),
        (enabled_count.to_string(), "启用中"),
        (disabled_count.to_string(), "已停用"),
        (format!("{:.2}×", avg_ratio), "平均倍率"),
        (custom_count.to_string(), "非基准倍率"),
    ];

    let filter_options = vec![
        format!("全部 ({total})"),
        format!("启用中 ({enabled_count})"),
        format!("已停用 ({disabled_count})"),
    ];

    // 过滤列表
    let filtered: Vec<GroupDto> = {
        let q = search().trim().to_lowercase();
        let tier = filter_tier();
        list.into_iter()
            .filter(|g| {
                if !q.is_empty()
                    && !g.name.to_lowercase().contains(&q)
                    && !g.remark.to_lowercase().contains(&q)
                {
                    return false;
                }
                match tier {
                    1 => g.status == 1,
                    2 => g.status != 1,
                    _ => true,
                }
            })
            .collect()
    };

    let open_new = move |_| {
        f_name.set(String::new());
        f_ratio.set("1.0".to_string());
        f_remark.set(String::new());
        modal_state.set(ModalState::New);
    };

    let mut open_edit = move |key: String| {
        if let Some(g) = groups().iter().find(|g| g.key == key) {
            f_name.set(g.name.clone());
            f_ratio.set(format!("{}", g.ratio));
            f_remark.set(g.remark.clone());
            modal_state.set(ModalState::Edit(key));
        }
    };

    // 写操作助手工厂:返回独立闭包,交给卡片(删除)。
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
                    WriteOp::Delete => delete_group_api(&client, &key).await,
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
    let write_delete = make_write();

    // 弹窗关闭并触发重拉
    let close_and_reload = move |_| {
        modal_state.set(ModalState::Closed);
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
                section { id: "groups-sec-stats", class: "scroll-mt-8 space-y-3",
                    h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
                    div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                        for (value, label) in stats {
                            StatCard { value, label }
                        }
                    }
                }

                // 2. 筛选与操作区
                section {
                    id: "groups-sec-filter",
                    class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
                    div { class: "flex items-center justify-between gap-3",
                        div { class: "flex items-center gap-2",
                            h2 { class: "text-sm font-medium text-zinc-300", "{SEC_FILTER}" }
                            span { class: "text-xs text-zinc-500", "按倍率分级或关键词筛选" }
                        }
                        div { class: "flex items-center gap-2",
                            button {
                                class: "rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-xs font-medium text-zinc-300 transition-colors hover:border-zinc-500 hover:text-white",
                                "data-testid": "refresh-groups",
                                onclick: move |_| reload.set(reload() + 1),
                                "刷新"
                            }
                            button {
                                class: "shrink-0 rounded-xl bg-white px-4 py-2 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                                onclick: open_new,
                                "✚ 新建分组"
                            }
                        }
                    }

                    input {
                        class: "w-full rounded-xl border border-zinc-700/80 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 outline-none transition focus:border-zinc-500",
                        r#type: "text",
                        placeholder: "搜索分组标识或备注...",
                        value: "{search}",
                        oninput: move |e| search.set(e.value()),
                    }

                    // 分类胶囊
                    div { class: "flex flex-wrap gap-2",
                        SegmentedCapsule {
                            items: filter_options,
                            active: filter_tier(),
                            on_select: move |i: usize| filter_tier.set(i),
                        }
                    }
                }

                // 3. 卡片网格区
                section { id: "groups-sec-list", class: "scroll-mt-8 space-y-4",
                    div { class: "flex items-center justify-between",
                        h2 { class: "text-lg font-medium text-zinc-100", "{SEC_LIST}" }
                        span { class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400",
                            if loading() { "加载中…" } else { "{filtered.len()} 组" }
                        }
                    }

                    if let Some(e) = err() {
                        div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 py-10 text-center",
                            p { class: "text-sm text-red-300", "加载分组失败" }
                            p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                            button {
                                class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                                onclick: move |_| reload.set(reload() + 1),
                                "重试"
                            }
                        }
                    } else if loading() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "正在加载分组…" }
                        }
                    } else if filtered.is_empty() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "没有匹配的分组" }
                        }
                    } else {
                        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                            "data-testid": "groups-list",
                            for g in filtered {
                                {
                                    let edit_key = g.key.clone();
                                    let delete_key = g.key.clone();
                                    let is_default = g.name == "default";
                                    rsx! {
                                        GroupCard {
                                            key: "{g.key}",
                                            group: g,
                                            is_default,
                                            on_edit: move |_| open_edit(edit_key.clone()),
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
            if matches!(modal_state(), ModalState::New | ModalState::Edit(_)) {
                GroupFormModal {
                    editing: matches!(modal_state(), ModalState::Edit(_)),
                    group_key: match modal_state() {
                        ModalState::Edit(k) => Some(k),
                        _ => None,
                    },
                    name: f_name,
                    ratio: f_ratio,
                    remark: f_remark,
                    on_cancel: move |_| modal_state.set(ModalState::Closed),
                    on_submit: close_and_reload,
                }
            }
    }
}

// ============ 组件 ============

#[component]
pub(crate) fn StatCard(value: String, label: &'static str) -> Element {
    rsx! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 px-4 py-3 transition-colors hover:border-zinc-600",
            p { class: "text-xl font-semibold tracking-tight text-white", "{value}" }
            p { class: "mt-0.5 text-xs text-zinc-500", "{label}" }
        }
    }
}

#[component]
pub(crate) fn Badge(text: String, tone: &'static str) -> Element {
    rsx! {
        span { class: "rounded-full border px-2 py-0.5 text-[11px] font-medium {tone}",
            "{text}"
        }
    }
}

/// 单个分组卡片 (对齐 UserCard 风格)
#[component]
fn GroupCard(
    group: GroupDto,
    is_default: bool,
    on_edit: EventHandler<()>,
    on_delete: EventHandler<()>,
) -> Element {
    let initial = group
        .name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();

    let m = group.ratio;

    // 倍率状态与徽标
    let (mult_badge_text, mult_badge_tone, bar_tone, bar_width_pct) = if (m - 1.0).abs() < 0.001 {
        (
            "基准 1.00×".to_string(),
            "border-zinc-700 bg-zinc-800/80 text-zinc-300",
            "bg-zinc-200",
            50,
        )
    } else if m < 1.0 {
        let discount = ((1.0 - m) * 100.0).round() as i64;
        (
            format!("优惠 {m:.2}× (-{discount}%)"),
            "border-emerald-500/30 bg-emerald-500/20 text-emerald-400",
            "bg-emerald-500",
            ((m / 2.0) * 100.0).clamp(10.0, 100.0) as u32,
        )
    } else {
        let markup = ((m - 1.0) * 100.0).round() as i64;
        (
            format!("溢价 {m:.2}× (+{markup}%)"),
            "border-amber-500/30 bg-amber-500/20 text-amber-400",
            "bg-amber-500",
            ((m / 2.0) * 100.0).clamp(10.0, 100.0) as u32,
        )
    };

    let status_text = if group.status == 1 {
        "启用中"
    } else {
        "已停用"
    };
    let status_tone = if group.status == 1 {
        "border-emerald-500/30 bg-emerald-500/20 text-emerald-400"
    } else {
        "border-zinc-700 bg-zinc-800/80 text-zinc-400"
    };

    let example_cost = (100.0 * m).round() as i64;
    let strategy_label = if m < 1.0 {
        "优惠折扣"
    } else if m > 1.0 {
        "溢价加价"
    } else {
        "标准基准"
    };

    rsx! {
        div { class: "group flex flex-col justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            "data-testid": "group-card",

            div { class: "space-y-3",
                // 头部:标识字母圈 + 分组名 + 默认标签
                div { class: "flex items-start gap-3",
                    div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-zinc-700 bg-zinc-800 text-sm font-semibold text-zinc-200 group-hover:border-zinc-500 transition-colors",
                        "{initial}"
                    }
                    div { class: "min-w-0 flex-1",
                        div { class: "flex items-center justify-between gap-2",
                            h3 { class: "truncate text-sm font-medium text-zinc-100", "{group.name}" }
                            if is_default {
                                span { class: "shrink-0 rounded bg-blue-950/60 border border-blue-800/60 px-1.5 py-0.5 text-[10px] font-mono text-blue-300",
                                    "默认"
                                }
                            } else {
                                span { class: "shrink-0 rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] font-mono text-zinc-400 border border-zinc-700/60",
                                    "#{group.key}"
                                }
                            }
                        }
                        p { class: "mt-0.5 truncate text-[11px] text-zinc-400", "{group.remark}" }
                    }
                }

                // 徽标行
                div { class: "flex flex-wrap gap-1.5",
                    Badge { text: mult_badge_text, tone: mult_badge_tone }
                    if is_default {
                        Badge { text: "系统内置".to_string(), tone: "border-blue-500/30 bg-blue-500/20 text-blue-300" }
                    } else {
                        Badge { text: "自定义分组".to_string(), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-400" }
                    }
                    Badge { text: status_text.to_string(), tone: status_tone }
                }

                // 倍率进度条
                div { class: "space-y-1.5",
                    div { class: "flex justify-between gap-2 text-[11px]",
                        span { class: "text-zinc-400", "计费倍率" }
                        span { class: "whitespace-nowrap font-medium text-zinc-200", "×{m:.2}" }
                    }
                    div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                        div { class: "h-full rounded-full {bar_tone} transition-all duration-300", style: "width: {bar_width_pct}%" }
                    }
                }

                // 详情指标行
                div { class: "space-y-1.5 text-xs pt-1",
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "费率模式" }
                        span { class: "font-medium text-zinc-200", "{strategy_label}" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "100额度实扣" }
                        span { class: "font-medium text-zinc-200 font-mono", "{example_cost} 点" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "调度作用域" }
                        span { class: "font-medium text-zinc-200", "全模型匹配" }
                    }
                }
            }

            // 底部操作按钮区 (严格对齐图 1: [编辑] [删除/内置])
            div { class: "mt-4 flex gap-1.5 border-t border-zinc-800 pt-3",
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white",
                    onclick: move |_| on_edit.call(()),
                    "编辑"
                }
                if is_default {
                    button {
                        class: "flex-1 rounded-lg border border-zinc-800 py-1 text-[11px] text-zinc-600 cursor-not-allowed",
                        title: "默认分组不能删除",
                        disabled: true,
                        "内置"
                    }
                } else {
                    button {
                        class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-red-400 transition-colors hover:bg-zinc-700 hover:text-red-300",
                        onclick: move |_| on_delete.call(()),
                        "删除"
                    }
                }
            }
        }
    }
}

// ============ 弹窗 ============

#[component]
pub(crate) fn Modal(title: String, on_close: EventHandler<()>, children: Element) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            onclick: move |_| on_close.call(()),
            div {
                class: "w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                div { class: "mb-5 flex items-center justify-between",
                    h3 { class: "text-base font-semibold text-zinc-100", "{title}" }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                        onclick: move |_| on_close.call(()),
                        "aria-label": "关闭",
                        svg {
                            class: "h-5 w-5",
                            fill: "none",
                            stroke: "currentColor",
                            view_box: "0 0 24 24",
                            stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18L18 6M6 6l12 12" }
                        }
                    }
                }
                {children}
            }
        }
    }
}

const MODAL_INPUT: &str = "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none";

#[component]
fn GroupFormModal(
    editing: bool,
    group_key: Option<String>,
    name: Signal<String>,
    ratio: Signal<String>,
    remark: Signal<String>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let title = if editing {
        "编辑分组"
    } else {
        "新建分组"
    };
    let submit_label = if editing {
        "保存修改"
    } else {
        "创建分组"
    };

    let parsed_ratio = ratio().trim().parse::<f64>().unwrap_or(1.0).max(0.0);

    let submitting = use_signal(|| false);

    // 工厂式复制,避免把原 signal 移动出闭包(供 rsx 中 submitting() 继续读取)
    let submitting2 = submitting;
    let on_submit2 = on_submit;
    let group_key2 = group_key.clone();
    let do_submit = move |_| {
        let key = group_key2.clone();
        let n = name.peek().trim().to_string();
        if n.is_empty() {
            return;
        }
        let r = ratio.peek().trim().parse::<f64>().unwrap_or(1.0).max(0.0);
        let rm = remark.peek().clone();
        let (mut sub, cb) = (submitting2, on_submit2);
        spawn(async move {
            sub.set(true);
            let client = ApiClient::shared().clone();
            let req = GroupUpsertRequest {
                name: n,
                ratio: r,
                model_whitelist: json!([]),
                remark: rm,
            };
            let res = match key {
                Some(kk) => update_group_api(&client, &kk, &req).await,
                None => create_group_api(&client, &req).await,
            };
            let _ = res;
            sub.set(false);
            cb.call(());
        });
    };

    let preset_ratios = [
        ("0.5× 半价", "0.5"),
        ("0.8× 优惠", "0.8"),
        ("1.0× 基准", "1.0"),
        ("1.2× 溢价", "1.2"),
        ("1.5× 高配", "1.5"),
        ("2.0× 双倍", "2.0"),
    ];

    rsx! {
        Modal { title: title.to_string(), on_close: move |_| on_cancel.call(()),
            div { class: "space-y-4",
                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "分组标识 (英文唯一标识)" }
                    input {
                        class: MODAL_INPUT,
                        placeholder: "例如: vip, claude, fast",
                        value: "{name}",
                        disabled: editing && name() == "default",
                        oninput: move |e| name.set(e.value()),
                    }
                    if editing && name() == "default" {
                        p { class: "mt-1 text-xs text-zinc-500", "默认分组标识不可更改" }
                    }
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "展示备注 (可选)" }
                    input {
                        class: MODAL_INPUT,
                        placeholder: "例如: VIP会员专线、高峰备用组",
                        value: "{remark}",
                        oninput: move |e| remark.set(e.value()),
                    }
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "计费倍率 (ratio ≥ 0)" }
                    input {
                        class: "{MODAL_INPUT} font-mono",
                        r#type: "text",
                        placeholder: "1.0",
                        value: "{ratio}",
                        oninput: move |e| ratio.set(e.value()),
                    }
                }

                // 快捷预设按钮
                div { class: "space-y-1.5",
                    p { class: "text-[11px] text-zinc-500", "快捷倍率预设" }
                    div { class: "flex flex-wrap gap-1.5",
                        for (lbl, val) in preset_ratios {
                            {
                                let is_active = (parsed_ratio - val.parse::<f64>().unwrap_or(0.0)).abs() < 0.001;
                                let btn_tone = if is_active {
                                    "border-zinc-100 bg-zinc-100 text-zinc-900 font-semibold"
                                } else {
                                    "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                                };
                                rsx! {
                                    button {
                                        class: "rounded-lg border px-2.5 py-1 text-xs transition-colors {btn_tone}",
                                        onclick: move |_| ratio.set(val.to_string()),
                                        "{lbl}"
                                    }
                                }
                            }
                        }
                    }
                }

                // 计费预览卡片
                div { class: "rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 text-xs space-y-1.5",
                    div { class: "flex justify-between text-zinc-400",
                        span { "标准消耗" }
                        span { "100 点额度" }
                    }
                    div { class: "flex justify-between font-medium",
                        span { class: "text-zinc-300", "该分组实际扣费" }
                        span { class: if parsed_ratio < 1.0 { "text-emerald-400" } else if parsed_ratio > 1.0 { "text-amber-400" } else { "text-zinc-200" },
                            "{(100.0 * parsed_ratio).round() as i64} 点额度"
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
                    class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200 disabled:opacity-40",
                    disabled: submitting(),
                    onclick: do_submit,
                    "{submit_label}"
                }
            }
        }
    }
}
