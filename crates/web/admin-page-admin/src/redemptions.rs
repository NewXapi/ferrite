//! 兑换码管理页:卡片式网格,对齐 GroupsPage / ChannelsPage / UsersPanel 规范。
//! 包含:顶部兑换码概览、综合筛选与批量生成、卡片网格、批量生成兑换码弹窗。

use dioxus::prelude::*;
use ui::SegmentedCapsule;

use crate::groups::{Badge, Modal, StatCard};
use crate::state::{EntityStore, RedRow};

/// 弹窗状态
#[derive(Clone, PartialEq)]
enum RedModalState {
    Closed,
    Generate,
}

const SEC_STATS: &str = "兑换码概览";
const SEC_FILTER: &str = "筛选与操作";
const SEC_LIST: &str = "兑换码列表";

#[component]
pub fn RedemptionsPage() -> Element {
    let store = use_context::<EntityStore>();
    let reds = store.redemptions;

    let mut search = use_signal(String::new);
    let mut filter_tier = use_signal(|| 0usize);
    let mut modal_state = use_signal(|| RedModalState::Closed);
    let mut copied_key = use_signal(|| None::<String>);

    // 生成弹窗字段
    let mut f_name = use_signal(String::new);
    let mut f_count = use_signal(|| "1".to_string());
    let mut f_quota = use_signal(|| "50".to_string());
    let mut f_expire_days = use_signal(String::new);

    let red_list = reds.read().clone();
    let total = red_list.len();
    let unused_count = red_list.iter().filter(|r| r.status == 1).count();
    let used_count = red_list.iter().filter(|r| r.status == 3).count();
    let disabled_count = red_list.iter().filter(|r| r.status == 2).count();

    let total_quota: f64 = red_list.iter().map(|r| r.quota).sum();
    let available_quota: f64 = red_list
        .iter()
        .filter(|r| r.status == 1)
        .map(|r| r.quota)
        .sum();

    let stats: [(String, &str); 5] = [
        (total.to_string(), "总兑换码数"),
        (unused_count.to_string(), "未使用可兑"),
        (used_count.to_string(), "已兑换核销"),
        (format!("¥{total_quota:.1}"), "发行总面值"),
        (format!("¥{available_quota:.1}"), "待兑现金额"),
    ];

    let filter_options = vec![
        format!("全部 ({total})"),
        format!("未使用 ({unused_count})"),
        format!("已兑换 ({used_count})"),
        format!("已停用 ({disabled_count})"),
    ];

    let filtered_indices: Vec<usize> = {
        let q = search().trim().to_lowercase();
        let tier = filter_tier();
        red_list
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                if !q.is_empty()
                    && !r.name.to_lowercase().contains(&q)
                    && !r.key.to_lowercase().contains(&q)
                {
                    return false;
                }
                match tier {
                    1 => r.status == 1,
                    2 => r.status == 3,
                    3 => r.status == 2,
                    _ => true,
                }
            })
            .map(|(i, _)| i)
            .collect()
    };

    let open_generate = move |_| {
        f_name.set("活动码".to_string());
        f_count.set("1".to_string());
        f_quota.set("50".to_string());
        f_expire_days.set(String::new());
        modal_state.set(RedModalState::Generate);
    };

    let toggle_status = move |idx: usize| {
        let mut rd = reds;
        if idx < rd.read().len() {
            let cur = rd.read()[idx].status;
            if cur != 3 {
                rd.write()[idx].status = if cur == 1 { 2 } else { 1 };
            }
        }
    };

    let delete_red = move |idx: usize| {
        let mut rd = reds;
        if idx < rd.read().len() {
            rd.write().remove(idx);
        }
    };

    let commit_generate = move |_| {
        let n = f_name.peek().trim().to_string();
        if n.is_empty() {
            return;
        }
        let cnt = f_count
            .peek()
            .trim()
            .parse::<u32>()
            .unwrap_or(1)
            .clamp(1, 100);
        let q = f_quota
            .peek()
            .trim()
            .parse::<f64>()
            .unwrap_or(10.0)
            .max(0.0);
        let days = f_expire_days.peek().trim().parse::<i64>().unwrap_or(0);

        let mut rd = reds;
        let base = rd.read().len() as u32;
        let upper = n.to_uppercase().replace(' ', "-");
        for k in 0..cnt {
            let code = (base.wrapping_add(k).wrapping_mul(2654435761) >> 4) % 65536;
            rd.write().push(RedRow {
                name: n.clone(),
                key: format!("{upper}-{code:04X}"),
                quota: q,
                status: 1,
                created: "2026-09-01".into(),
                expired: if days > 0 {
                    format!("{days} 天后")
                } else {
                    "永不过期".into()
                },
            });
        }
        modal_state.set(RedModalState::Closed);
    };

    rsx! {
            div { class: "flex flex-col gap-6",
                // 1. 概览统计区
                section { id: "reds-sec-stats", class: "scroll-mt-8 space-y-3",
                    h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
                    div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                        for (value, label) in stats {
                            StatCard { value, label }
                        }
                    }
                }

                // 2. 筛选与操作区
                section {
                    id: "reds-sec-filter",
                    class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
                    div { class: "flex items-center justify-between gap-3",
                        div { class: "flex items-center gap-2",
                            h2 { class: "text-sm font-medium text-zinc-300", "{SEC_FILTER}" }
                            span { class: "text-xs text-zinc-500", "按状态分级或活动标签筛选" }
                        }
                        button {
                            class: "shrink-0 rounded-xl bg-white px-4 py-2 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                            onclick: open_generate,
                            "✚ 生成兑换码"
                        }
                    }

                    input {
                        class: "w-full rounded-xl border border-zinc-700/80 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 outline-none transition focus:border-zinc-500",
                        r#type: "text",
                        placeholder: "搜索兑换码密文 (如 BETA-3F2A) 或活动名称...",
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
                section { id: "reds-sec-list", class: "scroll-mt-8 space-y-4",
                    div { class: "flex items-center justify-between",
                        h2 { class: "text-lg font-medium text-zinc-100", "{SEC_LIST}" }
                        span { class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400",
                            "{filtered_indices.len()} 张"
                        }
                    }

                    if filtered_indices.is_empty() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "没有匹配的兑换码" }
                        }
                    } else {
                        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                            for idx in filtered_indices {
                                {
                                    let r = reds.read()[idx].clone();
                                    let is_just_copied = copied_key() == Some(r.key.clone());
                                    rsx! {
                                        RedemptionCard {
                                            key: "{r.key}_{idx}",
                                            item: r,
                                            index: idx,
                                            is_just_copied: is_just_copied,
                                            on_copy: move |k: String| {
                                                copied_key.set(Some(k));
                                            },
                                            on_toggle: toggle_status,
                                            on_delete: delete_red,
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 生成弹窗
            if modal_state() == RedModalState::Generate {
                RedemptionGenerateModal {
                    name: f_name,
                    count: f_count,
                    quota: f_quota,
                    expire_days: f_expire_days,
                    on_cancel: move |_| modal_state.set(RedModalState::Closed),
                    on_submit: commit_generate,
                }
            }
    }
}

/// 兑换码卡片
#[component]
fn RedemptionCard(
    item: RedRow,
    index: usize,
    is_just_copied: bool,
    on_copy: EventHandler<String>,
    on_toggle: EventHandler<usize>,
    on_delete: EventHandler<usize>,
) -> Element {
    let initial = item
        .name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();

    let (status_text, status_tone, bar_tone, bar_pct) = match item.status {
        1 => (
            "未使用",
            "border-emerald-500/30 bg-emerald-500/20 text-emerald-400",
            "bg-emerald-500",
            100,
        ),
        2 => (
            "已停用",
            "border-amber-500/30 bg-amber-500/20 text-amber-400",
            "bg-amber-500",
            40,
        ),
        _ => (
            "已核销",
            "border-zinc-700 bg-zinc-800/80 text-zinc-400",
            "bg-zinc-700",
            0,
        ),
    };

    let key_clone = item.key.clone();

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
                            h3 { class: "truncate font-mono text-sm font-medium text-zinc-100", "{item.key}" }
                            span { class: "shrink-0 rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] font-mono text-zinc-400 border border-zinc-700/60",
                                "#{index + 1}"
                            }
                        }
                        p { class: "mt-0.5 truncate text-[11px] text-zinc-400", "{item.name}" }
                    }
                }

                // 徽标行
                div { class: "flex flex-wrap gap-1.5",
                    Badge { text: status_text.to_string(), tone: status_tone }
                    Badge { text: format!("面值 ¥{:.2}", item.quota), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-200 font-mono" }
                    Badge { text: item.expired.clone(), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-400" }
                }

                // 额度有效条
                div { class: "space-y-1.5",
                    div { class: "flex justify-between gap-2 text-[11px]",
                        span { class: "text-zinc-400", "可用面额" }
                        span { class: "whitespace-nowrap font-medium text-zinc-200 font-mono", "¥ {item.quota:.2}" }
                    }
                    div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                        div { class: "h-full rounded-full {bar_tone} transition-all duration-300", style: "width: {bar_pct}%" }
                    }
                }

                // 详情指标行
                div { class: "space-y-1.5 text-xs pt-1",
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "所属活动" }
                        span { class: "font-medium text-zinc-200", "{item.name}" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "生成日期" }
                        span { class: "font-mono text-zinc-400", "{item.created}" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "有效期" }
                        span { class: "font-medium text-zinc-300", "{item.expired}" }
                    }
                }
            }

            // 底部操作区 (标准三键布局: [复制卡密] [停用/启用] [删除])
            div { class: "mt-4 flex gap-1.5 border-t border-zinc-800 pt-3",
                button {
                    class: if is_just_copied {
                        "flex-1 rounded-lg border border-emerald-500/80 bg-emerald-950/60 py-1.5 text-xs text-emerald-300 transition-colors font-medium"
                    } else {
                        "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white"
                    },
                    onclick: move |_| on_copy.call(key_clone.clone()),
                    if is_just_copied { "已复制" } else { "复制卡密" }
                }
                if item.status != 3 {
                    button {
                        class: if item.status == 1 {
                            "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-amber-400 transition-colors hover:bg-zinc-700 hover:text-amber-300"
                        } else {
                            "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-emerald-400 transition-colors hover:bg-zinc-700 hover:text-emerald-300"
                        },
                        onclick: move |_| on_toggle.call(index),
                        if item.status == 1 { "停用" } else { "启用" }
                    }
                } else {
                    button {
                        class: "flex-1 rounded-lg border border-zinc-800 bg-zinc-900 py-1.5 text-xs text-zinc-600 cursor-not-allowed",
                        disabled: true,
                        "已核销"
                    }
                }
                button {
                    class: "w-7 rounded-lg border border-zinc-800 bg-zinc-800/40 py-1.5 text-xs text-zinc-500 hover:text-red-400 hover:border-red-900/60 transition-colors flex items-center justify-center",
                    title: "删除兑换码",
                    onclick: move |_| on_delete.call(index),
                    "✕"
                }
            }
        }
    }
}

/// 批量生成兑换码弹窗
#[component]
fn RedemptionGenerateModal(
    name: Signal<String>,
    count: Signal<String>,
    quota: Signal<String>,
    expire_days: Signal<String>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let parsed_count = count().trim().parse::<u32>().unwrap_or(1).clamp(1, 100);
    let parsed_quota = quota().trim().parse::<f64>().unwrap_or(10.0).max(0.0);
    let total_value = (parsed_count as f64) * parsed_quota;

    let preset_quotas = [
        ("¥10", "10"),
        ("¥20", "20"),
        ("¥50", "50"),
        ("¥100", "100"),
        ("¥200", "200"),
    ];

    let preset_expirations = [
        ("永不过期", ""),
        ("7 天", "7"),
        ("30 天", "30"),
        ("90 天", "90"),
        ("180 天", "180"),
    ];

    rsx! {
        Modal { title: "生成批量兑换码".to_string(), on_close: move |_| on_cancel.call(()),
            div { class: "space-y-4 max-h-[70vh] overflow-y-auto pr-1",
                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "活动/批次名称" }
                    input {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                        placeholder: "例如: 暑期充值赠送, 新人礼包",
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }

                div { class: "grid grid-cols-2 gap-3",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "生成数量 (1-100)" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                            r#type: "number",
                            min: "1",
                            max: "100",
                            value: "{count}",
                            oninput: move |e| count.set(e.value()),
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "单张面额 (元)" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                            placeholder: "50",
                            value: "{quota}",
                            oninput: move |e| quota.set(e.value()),
                        }
                    }
                }

                // 快捷面额按钮
                div { class: "space-y-1.5",
                    p { class: "text-[11px] text-zinc-500", "快捷面额预设" }
                    div { class: "flex flex-wrap gap-1.5",
                        for (lbl, val) in preset_quotas {
                            {
                                let is_active = (parsed_quota - val.parse::<f64>().unwrap_or(0.0)).abs() < 0.001;
                                let btn_tone = if is_active {
                                    "border-zinc-100 bg-zinc-100 text-zinc-900 font-semibold"
                                } else {
                                    "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                                };
                                rsx! {
                                    button {
                                        class: "rounded-lg border px-2.5 py-1 text-xs transition-colors {btn_tone}",
                                        onclick: move |_| quota.set(val.to_string()),
                                        "{lbl}"
                                    }
                                }
                            }
                        }
                    }
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "有效天数 (留空为永不过期)" }
                    input {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                        placeholder: "例如: 30",
                        value: "{expire_days}",
                        oninput: move |e| expire_days.set(e.value()),
                    }
                }

                // 快捷有效期按钮
                div { class: "space-y-1.5",
                    p { class: "text-[11px] text-zinc-500", "快捷有效期预设" }
                    div { class: "flex flex-wrap gap-1.5",
                        for (lbl, val) in preset_expirations {
                            {
                                let is_active = expire_days() == val;
                                let btn_tone = if is_active {
                                    "border-zinc-100 bg-zinc-100 text-zinc-900 font-semibold"
                                } else {
                                    "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                                };
                                rsx! {
                                    button {
                                        class: "rounded-lg border px-2.5 py-1 text-xs transition-colors {btn_tone}",
                                        onclick: move |_| expire_days.set(val.to_string()),
                                        "{lbl}"
                                    }
                                }
                            }
                        }
                    }
                }

                // 测算卡片
                div { class: "rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 text-xs space-y-1.5",
                    div { class: "flex justify-between text-zinc-400",
                        span { "生成总批次" }
                        span { "{parsed_count} 张卡密" }
                    }
                    div { class: "flex justify-between font-medium",
                        span { class: "text-zinc-300", "发行总面值金额" }
                        span { class: "text-emerald-400 font-mono text-sm",
                            "¥ {total_value:.2}"
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
                    "立即批量生成"
                }
            }
        }
    }
}
