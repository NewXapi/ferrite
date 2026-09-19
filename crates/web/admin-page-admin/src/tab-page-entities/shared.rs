use dioxus::prelude::*;

// —— 跨卡片共享文案 ——
pub const FIELD_DISPLAY: &str = "展示名";
pub const LBL_MULTIPLIER: &str = "倍率";
pub const BTN_NEW: &str = "新增";
pub const BTN_UPDATE: &str = "更新";
pub const BTN_CANCEL: &str = "取消";

// ============ 共享小件 ============

#[component]
pub fn CardPanel(
    section_index: usize,
    title: &'static str,
    hint: &'static str,
    count: usize,
    open: bool,
    on_toggle: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    let id = format!("ent-card-{section_index}");
    rsx! {
        section { id: "{id}", class: "shrink-0 overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900/60",
            button {
                class: "flex w-full items-center gap-2 px-4 py-2.5 text-left transition-colors hover:bg-zinc-900",
                onclick: move |e| on_toggle.call(e),
                span { class: "text-sm font-medium text-zinc-100", "{title}" }
                span { class: "rounded-full border border-zinc-700 px-1.5 text-[11px] text-zinc-400", "{count}" }
                span { class: "truncate text-[11px] text-zinc-600", "{hint}" }
            }
            if open {
                div { class: "space-y-3 border-t border-zinc-800 p-4", {children} }
            }
        }
    }
}

#[component]
pub fn NodeArea(children: Element) -> Element {
    rsx! {
        div { class: "min-h-[104px] rounded-lg border border-zinc-800 bg-zinc-950 p-3", {children} }
    }
}

#[component]
pub fn EmptyHint(text: &'static str) -> Element {
    rsx! {
        div { class: "flex h-full min-h-[72px] items-center justify-center",
            span { class: "text-[11px] text-zinc-600", "{text}" }
        }
    }
}

#[component]
pub fn EntityChip(
    label: String,
    sub: String,
    active: bool,
    on_pick: EventHandler<MouseEvent>,
    on_remove: EventHandler<MouseEvent>,
) -> Element {
    let tone = if active {
        "border-zinc-100 bg-zinc-100 text-zinc-900"
    } else {
        "border-zinc-700 bg-zinc-900 text-zinc-200 hover:border-zinc-500"
    };
    let sub_tone = if active {
        "text-zinc-600"
    } else {
        "text-zinc-500"
    };
    rsx! {
        span { class: "inline-flex items-center gap-1.5 rounded-full border py-1 pl-3 pr-1.5 transition-colors {tone}",
            button {
                class: "flex items-baseline gap-1.5",
                onclick: move |e| on_pick.call(e),
                span { class: "text-xs font-medium", "{label}" }
                if !sub.is_empty() {
                    span { class: "text-[11px] {sub_tone}", "{sub}" }
                }
            }
            button {
                class: "px-1 text-[11px] opacity-50 hover:text-red-400 hover:opacity-100",
                onclick: move |e| on_remove.call(e),
                "✕"
            }
        }
    }
}

#[component]
pub fn InputCell(
    label: &'static str,
    value: Signal<String>,
    placeholder: &'static str,
    #[props(default = false)] grow: bool,
) -> Element {
    let width = if grow { "min-w-[140px] flex-1" } else { "" };
    rsx! {
        label { class: "block space-y-1 {width}",
            span { class: "text-[11px] text-zinc-500", "{label}" }
            input {
                class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
                value: "{value.read()}",
                placeholder: "{placeholder}",
                oninput: move |e| value.set(e.value()),
            }
        }
    }
}

/// 原生下拉(移动端友好;样式与 InputCell/TextCell 一致)
#[component]
pub fn SelectCell(
    label: &'static str,
    value: String,
    options: &'static [&'static str],
    oninput: EventHandler<String>,
) -> Element {
    rsx! {
        label { class: "block space-y-1",
            span { class: "text-[11px] text-zinc-500", "{label}" }
            select {
                class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors focus:border-zinc-500",
                value: "{value}",
                oninput: move |e| oninput.call(e.value()),
                for opt in options {
                    option { value: "{opt}", selected: *opt == value, "{opt}" }
                }
            }
        }
    }
}

#[component]
pub fn TextCell(
    label: &'static str,
    value: String,
    placeholder: &'static str,
    oninput: EventHandler<String>,
) -> Element {
    rsx! {
        label { class: "block space-y-1",
            span { class: "text-[11px] text-zinc-500", "{label}" }
            input {
                class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
                value: "{value}",
                placeholder: "{placeholder}",
                oninput: move |e| oninput.call(e.value()),
            }
        }
    }
}

/// 非负单价解析:空/非法回退 0,负数归零
pub fn parse_nonneg(s: &str) -> f64 {
    s.trim().parse::<f64>().unwrap_or(0.0).max(0.0)
}

/// 倍率解析:空/非法回退 1.0,负数归零
pub fn parse_mult(s: &str) -> f64 {
    s.trim().parse::<f64>().unwrap_or(1.0).max(0.0)
}
