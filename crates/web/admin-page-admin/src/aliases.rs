//! 模型别名管理页:卡片式网格,对齐 GroupsPage 规范。

use dioxus::prelude::*;
use ui::SegmentedCapsule;

use crate::groups::{Badge, Modal, StatCard};
use crate::state::{AliasRow, EntityStore};

/// 弹窗状态
#[derive(Clone, PartialEq)]
enum AliasModalState {
    Closed,
    New,
    Edit(usize),
}

/// 别名管理页
#[component]
pub fn AliasesPage() -> Element {
    let store = use_context::<EntityStore>();
    let aliases = store.aliases;

    let mut search = use_signal(String::new);
    let mut filter_tier = use_signal(|| 0usize);
    let mut modal_state = use_signal(|| AliasModalState::Closed);

    let mut f_name = use_signal(String::new);
    let mut f_display = use_signal(String::new);
    let mut f_input = use_signal(|| "0.0175".to_string());
    let mut f_output = use_signal(|| "0.07".to_string());
    let mut f_mult = use_signal(|| "1.0".to_string());

    let alias_list = aliases.read().clone();
    let total = alias_list.len();
    let free_count = alias_list.iter().filter(|a| a.multiplier == 0.0).count();
    let standard_count = alias_list
        .iter()
        .filter(|a| (a.multiplier - 1.0).abs() < 0.001)
        .count();
    let custom_count = alias_list
        .iter()
        .filter(|a| a.multiplier != 0.0 && (a.multiplier - 1.0).abs() >= 0.001)
        .count();
    let avg_mult = if total > 0 {
        alias_list.iter().map(|a| a.multiplier).sum::<f64>() / (total as f64)
    } else {
        1.0
    };

    let stats: [(String, &str); 5] = [
        (total.to_string(), "总别名数"),
        (standard_count.to_string(), "标准 1.0× 别名"),
        (custom_count.to_string(), "自定倍率别名"),
        (free_count.to_string(), "免费别名 (0×)"),
        (format!("{:.2}×", avg_mult), "平均加价倍率"),
    ];

    let filter_options = vec![
        format!("全部 ({total})"),
        format!("标准 1.0× ({standard_count})"),
        format!("自定倍率 ({custom_count})"),
        format!("免费通道 ({free_count})"),
    ];

    let filtered_indices: Vec<usize> = {
        let q = search().trim().to_lowercase();
        let tier = filter_tier();
        alias_list
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                if !q.is_empty()
                    && !a.alias.to_lowercase().contains(&q)
                    && !a.display.to_lowercase().contains(&q)
                {
                    return false;
                }
                match tier {
                    1 => (a.multiplier - 1.0).abs() < 0.001,
                    2 => a.multiplier != 0.0 && (a.multiplier - 1.0).abs() >= 0.001,
                    3 => a.multiplier == 0.0,
                    _ => true,
                }
            })
            .map(|(i, _)| i)
            .collect()
    };

    let open_new = move |_| {
        f_name.set(String::new());
        f_display.set(String::new());
        f_input.set("0.0175".to_string());
        f_output.set("0.07".to_string());
        f_mult.set("1.0".to_string());
        modal_state.set(AliasModalState::New);
    };

    let open_edit = move |idx: usize| {
        if let Some(a) = aliases.read().get(idx) {
            f_name.set(a.alias.clone());
            f_display.set(a.display.clone());
            f_input.set(format!("{}", a.input_per_1k));
            f_output.set(format!("{}", a.output_per_1k));
            f_mult.set(format!("{}", a.multiplier));
            modal_state.set(AliasModalState::Edit(idx));
        }
    };

    let on_delete = move |idx: usize| {
        let mut a = aliases;
        if idx < a.read().len() {
            a.write().remove(idx);
        }
    };

    let commit_form = move |_| {
        let n = f_name.peek().trim().to_string();
        if n.is_empty() {
            return;
        }
        let d = f_display.peek().trim().to_string();
        let in_rate = f_input.peek().trim().parse::<f64>().unwrap_or(0.0).max(0.0);
        let out_rate = f_output
            .peek()
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0)
            .max(0.0);
        let m = f_mult.peek().trim().parse::<f64>().unwrap_or(1.0).max(0.0);

        let mut a = aliases;
        match *modal_state.peek() {
            AliasModalState::New => {
                let mut w = a.write();
                if let Some(pos) = w.iter().position(|item| item.alias == n) {
                    w[pos] = AliasRow {
                        alias: n,
                        display: d,
                        input_per_1k: in_rate,
                        output_per_1k: out_rate,
                        multiplier: m,
                    };
                } else {
                    w.push(AliasRow {
                        alias: n,
                        display: d,
                        input_per_1k: in_rate,
                        output_per_1k: out_rate,
                        multiplier: m,
                    });
                }
            }
            AliasModalState::Edit(idx) => {
                let mut w = a.write();
                if idx < w.len() {
                    w[idx] = AliasRow {
                        alias: n,
                        display: d,
                        input_per_1k: in_rate,
                        output_per_1k: out_rate,
                        multiplier: m,
                    };
                }
            }
            _ => {}
        }
        modal_state.set(AliasModalState::Closed);
    };

    rsx! {
            div { class: "flex flex-col gap-6",
                // 1. 统计区
                section { id: "aliases-sec-stats", class: "scroll-mt-8 space-y-3",
                    h2 { class: "text-lg font-medium text-zinc-100", "别名概览" }
                    div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                        for (value, label) in stats {
                            StatCard { value, label }
                        }
                    }
                }

                // 2. 筛选与操作区
                section {
                    id: "aliases-sec-filter",
                    class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
                    div { class: "flex items-center justify-between gap-3",
                        div { class: "flex items-center gap-2",
                            h2 { class: "text-sm font-medium text-zinc-300", "筛选与操作" }
                            span { class: "text-xs text-zinc-500", "按倍率与资费规则快速筛选" }
                        }
                        button {
                            class: "shrink-0 rounded-xl bg-white px-4 py-2 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                            onclick: open_new,
                            "✚ 新建别名"
                        }
                    }

                    input {
                        class: "w-full rounded-xl border border-zinc-700/80 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 outline-none transition focus:border-zinc-500",
                        r#type: "text",
                        placeholder: "搜索别名 ID 或展示名称 (如 gpt-4o, claude-sonnet)...",
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
                section { id: "aliases-sec-list", class: "scroll-mt-8 space-y-4",
                    div { class: "flex items-center justify-between",
                        h2 { class: "text-lg font-medium text-zinc-100", "别名列表" }
                        span { class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400",
                            "{filtered_indices.len()} 个"
                        }
                    }

                    if filtered_indices.is_empty() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "没有匹配的模型别名" }
                        }
                    } else {
                        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                            for idx in filtered_indices {
                                {
                                    let a = aliases.read()[idx].clone();
                                    rsx! {
                                        AliasCard {
                                            key: "{a.alias}",
                                            alias: a,
                                            index: idx,
                                            on_edit: open_edit,
                                            on_delete: on_delete,
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if matches!(modal_state(), AliasModalState::New | AliasModalState::Edit(_)) {
                AliasFormModal {
                    editing: matches!(modal_state(), AliasModalState::Edit(_)),
                    alias: f_name,
                    display: f_display,
                    input_rate: f_input,
                    output_rate: f_output,
                    multiplier: f_mult,
                    on_cancel: move |_| modal_state.set(AliasModalState::Closed),
                    on_submit: commit_form,
                }
            }
    }
}

/// 别名卡片
#[component]
fn AliasCard(
    alias: AliasRow,
    index: usize,
    on_edit: EventHandler<usize>,
    on_delete: EventHandler<usize>,
) -> Element {
    let initial = alias
        .alias
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();
    let m = alias.multiplier;

    let (mult_badge_text, mult_badge_tone, bar_tone, bar_width_pct) = if m == 0.0 {
        (
            "免费 0×".to_string(),
            "border-emerald-500/30 bg-emerald-500/20 text-emerald-400",
            "bg-emerald-500",
            15,
        )
    } else if (m - 1.0).abs() < 0.001 {
        (
            "标准 1.00×".to_string(),
            "border-zinc-700 bg-zinc-800/80 text-zinc-300",
            "bg-zinc-200",
            50,
        )
    } else if m < 1.0 {
        (
            format!("优惠 {m:.2}×"),
            "border-emerald-500/30 bg-emerald-500/20 text-emerald-400",
            "bg-emerald-500",
            ((m / 2.0) * 100.0).clamp(15.0, 100.0) as u32,
        )
    } else {
        (
            format!("溢价 {m:.2}×"),
            "border-amber-500/30 bg-amber-500/20 text-amber-400",
            "bg-amber-500",
            ((m / 2.0) * 100.0).clamp(15.0, 100.0) as u32,
        )
    };

    let display_title = if alias.display.is_empty() {
        alias.alias.clone()
    } else {
        alias.display.clone()
    };

    rsx! {
        div { class: "group flex flex-col justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            div { class: "space-y-3",
                div { class: "flex items-start gap-3",
                    div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-zinc-700 bg-zinc-800 text-sm font-semibold text-zinc-200 group-hover:border-zinc-500 transition-colors",
                        "{initial}"
                    }
                    div { class: "min-w-0 flex-1",
                        div { class: "flex items-center justify-between gap-2",
                            h3 { class: "truncate text-sm font-medium text-zinc-100", "{alias.alias}" }
                            span { class: "shrink-0 rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] font-mono text-zinc-400 border border-zinc-700/60",
                                "#{index + 1}"
                            }
                        }
                        p { class: "mt-0.5 truncate text-[11px] text-zinc-400", "{display_title}" }
                    }
                }

                div { class: "flex flex-wrap gap-1.5",
                    Badge { text: mult_badge_text, tone: mult_badge_tone }
                    Badge { text: format!("入: ¥{}/1k", alias.input_per_1k), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-300" }
                    Badge { text: format!("出: ¥{}/1k", alias.output_per_1k), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-300" }
                }

                div { class: "space-y-1.5",
                    div { class: "flex justify-between gap-2 text-[11px]",
                        span { class: "text-zinc-400", "计费倍率" }
                        span { class: "whitespace-nowrap font-medium text-zinc-200", "×{m:.2}" }
                    }
                    div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                        div { class: "h-full rounded-full {bar_tone} transition-all duration-300", style: "width: {bar_width_pct}%" }
                    }
                }

                div { class: "space-y-1.5 text-xs pt-1",
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "输入单价" }
                        span { class: "font-mono font-medium text-zinc-200", "¥ {alias.input_per_1k} / 1k" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "输出单价" }
                        span { class: "font-mono font-medium text-zinc-200", "¥ {alias.output_per_1k} / 1k" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "1M tokens 测算" }
                        span { class: "font-mono font-medium text-zinc-200",
                            "¥ {((alias.input_per_1k + alias.output_per_1k * 2.0) * 1000.0 * m).round() / 1000.0}"
                        }
                    }
                }
            }

            div { class: "mt-4 flex gap-1.5 border-t border-zinc-800 pt-3",
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white",
                    onclick: move |_| on_edit.call(index),
                    "编辑"
                }
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-red-400 transition-colors hover:bg-zinc-700 hover:text-red-300",
                    onclick: move |_| on_delete.call(index),
                    "删除"
                }
            }
        }
    }
}

/// 别名新建/编辑弹窗
#[component]
fn AliasFormModal(
    editing: bool,
    alias: Signal<String>,
    display: Signal<String>,
    input_rate: Signal<String>,
    output_rate: Signal<String>,
    multiplier: Signal<String>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let title = if editing {
        "编辑模型别名"
    } else {
        "新建模型别名"
    };
    let submit_label = if editing {
        "保存修改"
    } else {
        "创建别名"
    };

    rsx! {
        Modal { title: title.to_string(), on_close: move |_| on_cancel.call(()),
            div { class: "space-y-4",
                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "别名标识 (API 请求匹配名)" }
                    input {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                        placeholder: "例如: gpt-4o, claude-3-5-sonnet",
                        value: "{alias}",
                        oninput: move |e| alias.set(e.value()),
                    }
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "展示名称 (可选)" }
                    input {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                        placeholder: "例如: GPT-4o 旗舰模型",
                        value: "{display}",
                        oninput: move |e| display.set(e.value()),
                    }
                }

                div { class: "grid grid-cols-2 gap-3",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "输入单价 ¥/1k" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                            placeholder: "0.0175",
                            value: "{input_rate}",
                            oninput: move |e| input_rate.set(e.value()),
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "输出单价 ¥/1k" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                            placeholder: "0.07",
                            value: "{output_rate}",
                            oninput: move |e| output_rate.set(e.value()),
                        }
                    }
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "计费倍率 (multiplier ≥ 0)" }
                    input {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                        placeholder: "1.0",
                        value: "{multiplier}",
                        oninput: move |e| multiplier.set(e.value()),
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
