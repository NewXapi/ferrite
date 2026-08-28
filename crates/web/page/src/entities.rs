//! 实体设置页：分组 / 模型别名 / 渠道 三张可折叠卡片，各占一行。
//! 每张卡片上半是录入行，下半是该实体在拓扑里对应的节点内容。
//!
//! 数据在 `crate::store::EntityStore` 中，与拓扑抽屉共享：
//! 进这里改，抽屉里也能看到；反过来也成立。

use dioxus::prelude::*;

use crate::store::{ChannelRow, EntityStore};

#[component]
pub fn EntitiesPanel() -> Element {
    let mut open = use_signal(|| [true, true, true]);

    rsx! {
        div { class: "flex h-full min-h-0 flex-col gap-3 overflow-y-auto pr-1",
            GroupsCard {
                open: open()[0],
                on_toggle: move |_| { let mut o = open(); o[0] = !o[0]; open.set(o); },
            }
            AliasesCard {
                open: open()[1],
                on_toggle: move |_| { let mut o = open(); o[1] = !o[1]; open.set(o); },
            }
            ChannelsCard {
                open: open()[2],
                on_toggle: move |_| { let mut o = open(); o[2] = !o[2]; open.set(o); },
            }
        }
    }
}

// ============ 卡片 1：分组 ============

#[component]
fn GroupsCard(open: bool, on_toggle: EventHandler<MouseEvent>) -> Element {
    let store = use_context::<EntityStore>();
    let mut groups = store.groups;
    let mut name = use_signal(String::new);
    let mut display = use_signal(String::new);
    let mut editing = use_signal(|| None::<usize>);

    let commit = move |_| {
        let n = name.peek().trim().to_string();
        if n.is_empty() {
            return;
        }
        let d = display.peek().trim().to_string();
        match *editing.peek() {
            Some(i) => {
                groups.write()[i].name = n;
                groups.write()[i].display = d;
            }
            None => groups.write().push(crate::store::GroupRow {
                name: n,
                display: d,
            }),
        }
        name.set(String::new());
        display.set(String::new());
        editing.set(None);
    };

    rsx! {
        CardPanel {
            title: "分组",
            hint: "对模型别名分组；分组本身只有名字",
            count: groups.read().len(),
            open: open,
            on_toggle: on_toggle,

            div { class: "flex flex-wrap items-end gap-2",
                InputCell { label: "分组名", value: name, placeholder: "vip", grow: true }
                InputCell { label: "展示名", value: display, placeholder: "默认分组（可选）", grow: true }
                button {
                    class: "rounded-md border border-zinc-100 bg-zinc-100 px-3 py-1.5 text-xs font-medium text-zinc-900 hover:bg-zinc-300",
                    onclick: commit,
                    if editing().is_some() { "更新" } else { "新增" }
                }
                if editing().is_some() {
                    button {
                        class: "rounded-md border border-zinc-800 px-3 py-1.5 text-xs text-zinc-400 hover:border-zinc-600 hover:text-zinc-200",
                        onclick: move |_| {
                            editing.set(None);
                            name.set(String::new());
                            display.set(String::new());
                        },
                        "取消"
                    }
                }
            }

            NodeArea {
                if groups.read().is_empty() {
                    EmptyHint { text: "还没有分组" }
                } else {
                    div { class: "flex flex-wrap gap-2",
                        for (i, r) in groups.read().iter().enumerate() {
                            EntityChip {
                                label: r.name.clone(),
                                sub: r.display.clone(),
                                active: editing() == Some(i),
                                on_pick: move |_| {
                                    let r = groups.read()[i].clone();
                                    name.set(r.name);
                                    display.set(r.display);
                                    editing.set(Some(i));
                                },
                                on_remove: move |_| {
                                    groups.write().remove(i);
                                    if editing() == Some(i) { editing.set(None); }
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}

// ============ 卡片 2：模型别名 ============

#[component]
fn AliasesCard(open: bool, on_toggle: EventHandler<MouseEvent>) -> Element {
    let store = use_context::<EntityStore>();
    let mut aliases = store.aliases;
    let mut name = use_signal(String::new);
    let mut display = use_signal(String::new);
    let mut editing = use_signal(|| None::<usize>);

    let commit = move |_| {
        let n = name.peek().trim().to_string();
        if n.is_empty() {
            return;
        }
        let d = display.peek().trim().to_string();
        match *editing.peek() {
            Some(i) => {
                aliases.write()[i].alias = n;
                aliases.write()[i].display = d;
            }
            None => aliases.write().push(crate::store::AliasRow {
                alias: n,
                display: d,
            }),
        }
        name.set(String::new());
        display.set(String::new());
        editing.set(None);
    };

    rsx! {
        CardPanel {
            title: "模型别名",
            hint: "对外暴露给用户的模型名；卡牌样式后续再做",
            count: aliases.read().len(),
            open: open,
            on_toggle: on_toggle,

            div { class: "flex flex-wrap items-end gap-2",
                InputCell { label: "别名", value: name, placeholder: "gpt-4o", grow: true }
                InputCell { label: "展示名", value: display, placeholder: "GPT-4o（可选）", grow: true }
                button {
                    class: "rounded-md border border-zinc-100 bg-zinc-100 px-3 py-1.5 text-xs font-medium text-zinc-900 hover:bg-zinc-300",
                    onclick: commit,
                    if editing().is_some() { "更新" } else { "新增" }
                }
                if editing().is_some() {
                    button {
                        class: "rounded-md border border-zinc-800 px-3 py-1.5 text-xs text-zinc-400 hover:border-zinc-600 hover:text-zinc-200",
                        onclick: move |_| {
                            editing.set(None);
                            name.set(String::new());
                            display.set(String::new());
                        },
                        "取消"
                    }
                }
            }

            NodeArea {
                if aliases.read().is_empty() {
                    EmptyHint { text: "还没有模型别名" }
                } else {
                    div { class: "flex flex-wrap gap-2",
                        for (i, r) in aliases.read().iter().enumerate() {
                            EntityChip {
                                label: r.alias.clone(),
                                sub: r.display.clone(),
                                active: editing() == Some(i),
                                on_pick: move |_| {
                                    let r = aliases.read()[i].clone();
                                    name.set(r.alias);
                                    display.set(r.display);
                                    editing.set(Some(i));
                                },
                                on_remove: move |_| {
                                    aliases.write().remove(i);
                                    if editing() == Some(i) { editing.set(None); }
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}

// ============ 卡片 3：渠道 ============

#[component]
fn ChannelsCard(open: bool, on_toggle: EventHandler<MouseEvent>) -> Element {
    let store = use_context::<EntityStore>();
    let mut channels = store.channels;
    let mut current = use_signal(|| 0usize);
    let idx = current();
    let row = channels.read().get(idx).cloned();

    rsx! {
        CardPanel {
            title: "渠道",
            hint: "URL + Key 是凭证容器；调度模型由候补池加入，名字不可改",
            count: channels.read().len(),
            open: open,
            on_toggle: on_toggle,

            div { class: "flex flex-wrap items-center gap-2",
                for (i, c) in channels.read().iter().enumerate() {
                    {
                        let label = c.name.clone();
                        let active = idx == i;
                        let tone = if active {
                            "border-zinc-100 bg-zinc-100 text-zinc-900"
                        } else {
                            "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                        };
                        rsx! {
                            button {
                                class: "rounded-full border px-3 py-1 text-xs font-medium transition-colors {tone}",
                                onclick: move |_| current.set(i),
                                "{label}"
                            }
                        }
                    }
                }
                button {
                    class: "rounded-full border border-dashed border-zinc-700 px-3 py-1 text-xs text-zinc-500 hover:border-zinc-500 hover:text-zinc-300",
                    onclick: move |_| {
                        channels.write().push(ChannelRow {
                            name: "新渠道".into(),
                            url: String::new(),
                            keys: String::new(),
                            candidates: vec![],
                            dispatch: vec![],
                        });
                        let last = channels.read().len() - 1;
                        current.set(last);
                    },
                    "＋ 新建渠道"
                }
                if channels.read().len() > 1 {
                    button {
                        class: "ml-auto rounded-md border border-zinc-800 px-2.5 py-1 text-xs text-zinc-400 hover:border-red-700 hover:text-red-400",
                        onclick: move |_| {
                            channels.write().remove(idx);
                            current.set(0);
                        },
                        "删除此渠道"
                    }
                }
            }

            if let Some(c) = row {
                div { class: "grid grid-cols-1 gap-2 sm:grid-cols-3",
                    TextCell {
                        label: "渠道名称",
                        value: c.name.clone(),
                        placeholder: "OpenAI 官方",
                        oninput: move |v: String| channels.write()[idx].name = v,
                    }
                    TextCell {
                        label: "Base URL",
                        value: c.url.clone(),
                        placeholder: "https://…",
                        oninput: move |v: String| channels.write()[idx].url = v,
                    }
                    TextCell {
                        label: "API Key",
                        value: c.keys.clone(),
                        placeholder: "sk-…",
                        oninput: move |v: String| channels.write()[idx].keys = v,
                    }
                }

                NodeArea {
                    div { class: "grid min-h-0 grid-cols-1 gap-3 lg:grid-cols-2",
                        // 候补池
                        div { class: "flex min-h-0 flex-col gap-2 rounded-lg border border-dashed border-zinc-700 bg-zinc-950/60 p-3",
                            div { class: "flex items-center justify-between gap-2",
                                div {
                                    p { class: "text-xs text-zinc-300", "候补池" }
                                    p { class: "text-[11px] text-zinc-600", "拉取结果，尚未进入拓扑" }
                                }
                                button {
                                    class: "rounded border border-zinc-700 px-2 py-0.5 text-[11px] text-zinc-300 hover:border-zinc-500",
                                    onclick: move |_| {
                                        // mock 拉取：只补上游存在但本地没有的名字
                                        let pool = ["gpt-4o", "gpt-4o-mini", "gpt-5", "o3", "o3-mini"];
                                        let mut w = channels.write();
                                        let c2 = &mut w[idx];
                                        let have: Vec<String> = c2
                                            .candidates
                                            .iter()
                                            .map(|(n, _)| n.clone())
                                            .chain(c2.dispatch.iter().cloned())
                                            .collect();
                                        for m in pool {
                                            if !have.iter().any(|x| x == m) {
                                                c2.candidates.push((m.to_string(), false));
                                            }
                                        }
                                    },
                                    "拉取模型"
                                }
                            }
                            if c.candidates.is_empty() {
                                EmptyHint { text: "点「拉取模型」获取候补" }
                            } else {
                                div { class: "min-h-0 flex-1 space-y-0.5 overflow-y-auto",
                                    for (j, (m, on)) in c.candidates.iter().enumerate() {
                                        {
                                            let label = m.clone();
                                            let checked = *on;
                                            rsx! {
                                                label { class: "flex cursor-pointer items-center gap-2 rounded px-2 py-1 hover:bg-zinc-900",
                                                    input {
                                                        r#type: "checkbox",
                                                        class: "accent-zinc-100",
                                                        checked: checked,
                                                        onchange: move |_| {
                                                            let mut w = channels.write();
                                                            let v = w[idx].candidates[j].1;
                                                            w[idx].candidates[j].1 = !v;
                                                        },
                                                    }
                                                    span { class: "font-mono text-xs text-zinc-400", "{label}" }
                                                }
                                            }
                                        }
                                    }
                                }
                                div { class: "flex gap-1.5 border-t border-zinc-800 pt-2",
                                    button {
                                        class: "rounded border border-zinc-100 bg-zinc-100 px-2 py-0.5 text-[11px] font-medium text-zinc-900 hover:bg-zinc-300",
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
                                        "加入调度 →"
                                    }
                                    button {
                                        class: "rounded border border-zinc-800 px-2 py-0.5 text-[11px] text-zinc-500 hover:border-zinc-600 hover:text-zinc-300",
                                        onclick: move |_| { channels.write()[idx].candidates.clear(); },
                                        "清空候补"
                                    }
                                }
                            }
                        }

                        // 调度模型
                        div { class: "flex min-h-0 flex-col gap-2 rounded-lg border border-zinc-800 bg-zinc-950 p-3",
                            div {
                                p { class: "text-xs text-zinc-300", "调度模型" }
                                p { class: "text-[11px] text-zinc-600", "已在拓扑中；名字来自上游，不可改" }
                            }
                            if c.dispatch.is_empty() {
                                EmptyHint { text: "从左侧候补池加入" }
                            } else {
                                div { class: "min-h-0 flex-1 space-y-1 overflow-y-auto",
                                    for (j, m) in c.dispatch.iter().enumerate() {
                                        {
                                            let label = m.clone();
                                            rsx! {
                                                div { class: "flex items-center justify-between gap-2 rounded border border-zinc-800 bg-zinc-900 px-2 py-1",
                                                    span { class: "truncate font-mono text-xs text-zinc-200", "{label}" }
                                                    button {
                                                        class: "shrink-0 text-[11px] text-zinc-600 hover:text-red-400",
                                                        title: "移出拓扑，退回候补池",
                                                        onclick: move |_| {
                                                            let mut w = channels.write();
                                                            let m = w[idx].dispatch.remove(j);
                                                            w[idx].candidates.push((m, false));
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
        }
    }
}

// ============ 共享小件 ============

#[component]
fn CardPanel(
    title: &'static str,
    hint: &'static str,
    count: usize,
    open: bool,
    on_toggle: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    rsx! {
        section { class: "shrink-0 overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900/60",
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
fn NodeArea(children: Element) -> Element {
    rsx! {
        div { class: "min-h-[104px] rounded-lg border border-zinc-800 bg-zinc-950 p-3", {children} }
    }
}

#[component]
fn EmptyHint(text: &'static str) -> Element {
    rsx! {
        div { class: "flex h-full min-h-[72px] items-center justify-center",
            span { class: "text-[11px] text-zinc-600", "{text}" }
        }
    }
}

#[component]
fn EntityChip(
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
fn InputCell(
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

#[component]
fn TextCell(
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
