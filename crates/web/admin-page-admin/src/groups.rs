//! 分组管理页:卡片式设计,对齐用户管理面板 (UsersPanel) 视觉规范。
//! 包含:顶部概览指标、快捷筛选与新建、卡片网格与底部操作区、新建/编辑与快捷调倍率弹窗。

use dioxus::prelude::*;
use ui::SegmentedCapsule;

use crate::state::{EntityStore, GroupRow};

/// 弹窗状态
#[derive(Clone, PartialEq)]
enum ModalState {
    Closed,
    New,
    Edit(usize),
    QuickMult(usize),
}

const SEC_STATS: &str = "分组概览";
const SEC_FILTER: &str = "筛选与操作";
const SEC_LIST: &str = "分组列表";

#[component]
pub fn GroupsPage() -> Element {
    let store = use_context::<EntityStore>();
    let groups = store.groups;

    let mut search = use_signal(String::new);
    let mut filter_tier = use_signal(|| 0usize);
    let mut modal_state = use_signal(|| ModalState::Closed);

    // 表单状态
    let mut f_name = use_signal(String::new);
    let mut f_display = use_signal(String::new);
    let mut f_mult = use_signal(|| "1.0".to_string());

    // 快捷调倍率状态
    let mut q_mult = use_signal(|| "1.0".to_string());

    let group_list = groups.read().clone();
    let total = group_list.len();
    let base_count = group_list
        .iter()
        .filter(|g| (g.multiplier - 1.0).abs() < 0.001)
        .count();
    let discount_count = group_list.iter().filter(|g| g.multiplier < 0.999).count();
    let premium_count = group_list.iter().filter(|g| g.multiplier > 1.001).count();
    let avg_mult = if total > 0 {
        group_list.iter().map(|g| g.multiplier).sum::<f64>() / (total as f64)
    } else {
        1.0
    };

    let stats: [(String, &str); 5] = [
        (total.to_string(), "总分组数"),
        (base_count.to_string(), "基准倍率 (1.0×)"),
        (discount_count.to_string(), "优惠分组 (<1.0×)"),
        (premium_count.to_string(), "溢价分组 (>1.0×)"),
        (format!("{:.2}×", avg_mult), "平均倍率"),
    ];

    let filter_options = vec![
        format!("全部 ({total})"),
        format!("基准 1.0× ({base_count})"),
        format!("优惠折扣 ({discount_count})"),
        format!("溢价加成 ({premium_count})"),
    ];

    // 过滤列表
    let filtered_indices: Vec<usize> = {
        let q = search().trim().to_lowercase();
        let tier = filter_tier();
        group_list
            .iter()
            .enumerate()
            .filter(|(_, g)| {
                // 文本匹配
                if !q.is_empty()
                    && !g.name.to_lowercase().contains(&q)
                    && !g.display.to_lowercase().contains(&q)
                {
                    return false;
                }
                // 分类匹配
                match tier {
                    1 => (g.multiplier - 1.0).abs() < 0.001,
                    2 => g.multiplier < 0.999,
                    3 => g.multiplier > 1.001,
                    _ => true,
                }
            })
            .map(|(i, _)| i)
            .collect()
    };

    let open_new = move |_| {
        f_name.set(String::new());
        f_display.set(String::new());
        f_mult.set("1.0".to_string());
        modal_state.set(ModalState::New);
    };

    let open_edit = move |idx: usize| {
        if let Some(g) = groups.read().get(idx) {
            f_name.set(g.name.clone());
            f_display.set(g.display.clone());
            f_mult.set(format!("{}", g.multiplier));
            modal_state.set(ModalState::Edit(idx));
        }
    };

    let open_quick_mult = move |idx: usize| {
        if let Some(g) = groups.read().get(idx) {
            q_mult.set(format!("{}", g.multiplier));
            modal_state.set(ModalState::QuickMult(idx));
        }
    };

    let on_delete = move |idx: usize| {
        if let Some(g) = groups.read().get(idx) {
            // 禁止删除 default 默认分组
            if g.name == "default" {
                return;
            }
        }
        let mut g = groups;
        g.write().remove(idx);
    };

    let commit_form = move |_| {
        let n = f_name.peek().trim().to_string();
        if n.is_empty() {
            return;
        }
        let d = f_display.peek().trim().to_string();
        let m = f_mult.peek().trim().parse::<f64>().unwrap_or(1.0).max(0.0);

        let mut g = groups;
        match *modal_state.peek() {
            ModalState::New => {
                // 如果已存在同名，覆盖更新，否则追加
                let mut w = g.write();
                if let Some(pos) = w.iter().position(|item| item.name == n) {
                    w[pos] = GroupRow {
                        name: n,
                        display: d,
                        multiplier: m,
                    };
                } else {
                    w.push(GroupRow {
                        name: n,
                        display: d,
                        multiplier: m,
                    });
                }
            }
            ModalState::Edit(idx) => {
                let mut w = g.write();
                if idx < w.len() {
                    w[idx] = GroupRow {
                        name: n,
                        display: d,
                        multiplier: m,
                    };
                }
            }
            _ => {}
        }
        modal_state.set(ModalState::Closed);
    };

    let mut quick_mult_target_idx = 0usize;
    if let ModalState::QuickMult(idx) = modal_state() {
        quick_mult_target_idx = idx;
    }

    let commit_quick_mult = move |_| {
        let idx = quick_mult_target_idx;
        let m = q_mult.peek().trim().parse::<f64>().unwrap_or(1.0).max(0.0);
        let mut g = groups;
        if idx < g.read().len() {
            g.write()[idx].multiplier = m;
        }
        modal_state.set(ModalState::Closed);
    };

    rsx! {
            div { class: "flex flex-col gap-6",
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
                        button {
                            class: "shrink-0 rounded-xl bg-white px-4 py-2 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                            onclick: open_new,
                            "✚ 新建分组"
                        }
                    }

                    input {
                        class: "w-full rounded-xl border border-zinc-700/80 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 outline-none transition focus:border-zinc-500",
                        r#type: "text",
                        placeholder: "搜索分组标识或展示名 (如 vip, default, claude)...",
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
                            "{filtered_indices.len()} 组"
                        }
                    }

                    if filtered_indices.is_empty() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "没有匹配的分组" }
                        }
                    } else {
                        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                            for idx in filtered_indices {
                                {
                                    let g = groups.read()[idx].clone();
                                    rsx! {
                                        GroupCard {
                                            key: "{g.name}",
                                            group: g,
                                            index: idx,
                                            on_edit: open_edit,
                                            on_quick_mult: open_quick_mult,
                                            on_delete: on_delete,
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
                    name: f_name,
                    display: f_display,
                    multiplier: f_mult,
                    on_cancel: move |_| modal_state.set(ModalState::Closed),
                    on_submit: commit_form,
                }
            }

            // 快捷调倍率弹窗
            if let ModalState::QuickMult(idx) = modal_state() {
                if let Some(g) = groups.read().get(idx).cloned() {
                    QuickMultModal {
                        group: g,
                        multiplier: q_mult,
                        on_cancel: move |_| modal_state.set(ModalState::Closed),
                        on_submit: commit_quick_mult,
                    }
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
    group: GroupRow,
    index: usize,
    on_edit: EventHandler<usize>,
    on_quick_mult: EventHandler<usize>,
    on_delete: EventHandler<usize>,
) -> Element {
    let initial = group
        .name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();

    let m = group.multiplier;
    let is_default = group.name == "default";

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

    let display_title = if group.display.is_empty() {
        group.name.clone()
    } else {
        group.display.clone()
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

            div { class: "space-y-3",
                // 头部:标识字母圈 + 分组名 + 序号/默认标签
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
                                    "#{index + 1}"
                                }
                            }
                        }
                        p { class: "mt-0.5 truncate text-[11px] text-zinc-400", "{display_title}" }
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
                    Badge { text: "group_ratio".to_string(), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-500" }
                }

                // 倍率进度条 (参考额度进度条)
                div { class: "space-y-1.5",
                    div { class: "flex justify-between gap-2 text-[11px]",
                        span { class: "text-zinc-400", "计费倍率" }
                        span { class: "whitespace-nowrap font-medium text-zinc-200", "×{m:.2}" }
                    }
                    div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                        div { class: "h-full rounded-full {bar_tone} transition-all duration-300", style: "width: {bar_width_pct}%" }
                    }
                }

                // 详情指标行 (参考用户卡片的数据行)
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
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "别名覆盖" }
                        span { class: "font-medium text-zinc-200", "支持" }
                    }
                }
            }

            // 底部操作按钮区 (严格对齐图 1: [编辑] [调倍率] [删除/内置])
            div { class: "mt-4 flex gap-1.5 border-t border-zinc-800 pt-3",
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white",
                    onclick: move |_| on_edit.call(index),
                    "编辑"
                }
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-emerald-400 transition-colors hover:bg-zinc-700 hover:text-emerald-300",
                    onclick: move |_| on_quick_mult.call(index),
                    "调倍率"
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
                        onclick: move |_| on_delete.call(index),
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
    name: Signal<String>,
    display: Signal<String>,
    multiplier: Signal<String>,
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

    let parsed_mult = multiplier().trim().parse::<f64>().unwrap_or(1.0).max(0.0);

    let preset_mults = [
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
                    label { class: "mb-1.5 block text-xs text-zinc-400", "展示名称 (可选)" }
                    input {
                        class: MODAL_INPUT,
                        placeholder: "例如: VIP会员专线、高峰备用组",
                        value: "{display}",
                        oninput: move |e| display.set(e.value()),
                    }
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "计费倍率 (multiplier ≥ 0)" }
                    input {
                        class: "{MODAL_INPUT} font-mono",
                        r#type: "text",
                        placeholder: "1.0",
                        value: "{multiplier}",
                        oninput: move |e| multiplier.set(e.value()),
                    }
                }

                // 快捷预设按钮
                div { class: "space-y-1.5",
                    p { class: "text-[11px] text-zinc-500", "快捷倍率预设" }
                    div { class: "flex flex-wrap gap-1.5",
                        for (lbl, val) in preset_mults {
                            {
                                let is_active = (parsed_mult - val.parse::<f64>().unwrap_or(0.0)).abs() < 0.001;
                                let btn_tone = if is_active {
                                    "border-zinc-100 bg-zinc-100 text-zinc-900 font-semibold"
                                } else {
                                    "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                                };
                                rsx! {
                                    button {
                                        class: "rounded-lg border px-2.5 py-1 text-xs transition-colors {btn_tone}",
                                        onclick: move |_| multiplier.set(val.to_string()),
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
                        span { class: if parsed_mult < 1.0 { "text-emerald-400" } else if parsed_mult > 1.0 { "text-amber-400" } else { "text-zinc-200" },
                            "{(100.0 * parsed_mult).round() as i64} 点额度"
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

/// 快捷调倍率弹窗
#[component]
fn QuickMultModal(
    group: GroupRow,
    multiplier: Signal<String>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let parsed_mult = multiplier().trim().parse::<f64>().unwrap_or(1.0).max(0.0);

    let preset_mults = [
        ("0.5×", "0.5"),
        ("0.8×", "0.8"),
        ("1.0×", "1.0"),
        ("1.2×", "1.2"),
        ("1.5×", "1.5"),
        ("2.0×", "2.0"),
    ];

    rsx! {
        Modal { title: format!("调整倍率 - {}", group.name), on_close: move |_| on_cancel.call(()),
            div { class: "space-y-4",
                div { class: "rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 text-xs space-y-1",
                    div { class: "flex justify-between",
                        span { class: "text-zinc-400", "目标分组" }
                        span { class: "font-medium text-zinc-200", "{group.name}" }
                    }
                    div { class: "flex justify-between",
                        span { class: "text-zinc-400", "当前倍率" }
                        span { class: "font-medium text-zinc-200 font-mono", "×{group.multiplier:.2}" }
                    }
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "新倍率系数" }
                    input {
                        class: "{MODAL_INPUT} font-mono",
                        r#type: "text",
                        value: "{multiplier}",
                        oninput: move |e| multiplier.set(e.value()),
                    }
                }

                div { class: "flex flex-wrap gap-2",
                    for (lbl, val) in preset_mults {
                        {
                            let is_active = (parsed_mult - val.parse::<f64>().unwrap_or(0.0)).abs() < 0.001;
                            let btn_tone = if is_active {
                                "border-zinc-100 bg-zinc-100 text-zinc-900 font-semibold"
                            } else {
                                "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                            };
                            rsx! {
                                button {
                                    class: "flex-1 min-w-[28%] rounded-lg border py-2 text-xs transition-colors {btn_tone}",
                                    onclick: move |_| multiplier.set(val.to_string()),
                                    "{lbl}"
                                }
                            }
                        }
                    }
                }

                div { class: "flex justify-between items-center rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 text-xs",
                    span { class: "text-zinc-400", "100 额度消耗测算" }
                    span { class: "font-medium font-mono text-sm",
                        class: if parsed_mult < 1.0 { "text-emerald-400" } else if parsed_mult > 1.0 { "text-amber-400" } else { "text-zinc-200" },
                        "{(100.0 * parsed_mult).round() as i64} 点"
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
                    "确认调整"
                }
            }
        }
    }
}
