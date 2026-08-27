//! 实体设置页：分组 / 模型别名 / 渠道 三块可折叠卡片，各占一行。
//! 每张卡片上半是录入行，下半是对应的节点内容。
//!
//! 三者的编辑重量不同，卡片内的形态也不同：
//! - 分组、模型别名：只有名字，一行录入 + 芯片列表
//! - 渠道：名称 + URL + Key 三输入，外加「候补池 → 调度模型」双列多选
//!
//! 术语（与拓扑图一致）：
//! - 模型别名：对外暴露给用户的模型名
//! - 调度模型：渠道下真实可用的上游模型，**不可改名**
//! - 候补池：从渠道拉取回来但尚未加入拓扑的临时列表
//!
//! 数据 mock，改动仅存在于前端内存。

use dioxus::prelude::*;

#[component]
pub fn EntitiesPanel() -> Element {
    // 三张卡片的折叠态，默认全开。
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
    // (分组名, 展示名)
    let mut groups = use_signal(|| {
        vec![
            ("default".to_string(), "默认分组".to_string()),
            ("vip".to_string(), "VIP".to_string()),
            ("claude".to_string(), "Claude 专用".to_string()),
            ("trial".to_string(), "试用".to_string()),
        ]
    });
    let mut name = use_signal(String::new);
    let mut display = use_signal(String::new);
    // 正在编辑的行；None 表示录入新项。
    let mut editing = use_signal(|| None::<usize>);

    let commit = move |_| {
        let n = name.peek().trim().to_string();
        if n.is_empty() {
            return;
        }
        let d = display.peek().trim().to_string();
        match *editing.peek() {
            Some(i) => groups.write()[i] = (n, d),
            None => groups.write().push((n, d)),
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

            // 录入行
            div { class: "flex flex-wrap items-end gap-2",
                InputCell { label: "分组名", value: name, placeholder: "vip", grow: true }
                InputCell { label: "展示名", value: display, placeholder: "VIP（可选）", grow: true }
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

            // 节点内容
            NodeArea {
                if groups.read().is_empty() {
                    EmptyHint { text: "还没有分组" }
                } else {
                    div { class: "flex flex-wrap gap-2",
                        for (i, (n, d)) in groups.read().iter().enumerate() {
                            EntityChip {
                                label: n.clone(),
                                sub: d.clone(),
                                active: editing() == Some(i),
                                on_pick: move |_| {
                                    let (n, d) = groups.read()[i].clone();
                                    name.set(n);
                                    display.set(d);
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
    // (别名, 展示名)
    let mut aliases = use_signal(|| {
        vec![
            ("gpt-4o".to_string(), "GPT-4o".to_string()),
            ("gpt-5".to_string(), "GPT-5".to_string()),
            ("claude-sonnet-4".to_string(), "Claude Sonnet 4".to_string()),
            ("gemini-2.5-pro".to_string(), "Gemini 2.5 Pro".to_string()),
        ]
    });
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
            Some(i) => aliases.write()[i] = (n, d),
            None => aliases.write().push((n, d)),
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
                        for (i, (n, d)) in aliases.read().iter().enumerate() {
                            EntityChip {
                                label: n.clone(),
                                sub: d.clone(),
                                active: editing() == Some(i),
                                on_pick: move |_| {
                                    let (n, d) = aliases.read()[i].clone();
                                    name.set(n);
                                    display.set(d);
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

#[derive(Clone, PartialEq)]
struct ChannelRow {
    name: String,
    url: String,
    keys: String,
    /// 拉取回来的候补池：(模型名, 是否勾选)
    candidates: Vec<(String, bool)>,
    /// 已加入拓扑的调度模型；名字来自上游，不可改名
    dispatch: Vec<String>,
}

#[component]
fn ChannelsCard(open: bool, on_toggle: EventHandler<MouseEvent>) -> Element {
    let mut channels = use_signal(|| {
        vec![
            ChannelRow {
                name: "OpenAI 官方".into(),
                url: "https://api.openai.com/v1".into(),
                keys: "sk-**************************".into(),
                candidates: vec![
                    ("o3".to_string(), false),
                    ("o3-mini".to_string(), false),
                    ("text-embedding-3-large".to_string(), false),
                ],
                dispatch: vec!["gpt-4o-2024-11-20".into(), "gpt-4o-mini".into()],
            },
            ChannelRow {
                name: "Azure East".into(),
                url: "https://east.azure.example/openai".into(),
                keys: "az-****".into(),
                candidates: vec![("gpt-4o".to_string(), false)],
                dispatch: vec!["gpt-4o".into()],
            },
            ChannelRow {
                name: "OneAPI 上游".into(),
                url: "https://oneapi.example/v1".into(),
                keys: "oa-****".into(),
                candidates: vec![],
                dispatch: vec!["gpt-4o".into(), "gpt-5".into(), "claude-sonnet-4".into()],
            },
        ]
    });
    let mut current = use_signal(|| 0usize);

    // 录入区绑定到当前选中渠道；新建时 current 指向新行。
    let idx = current();
    let row = channels.read().get(idx).cloned();

    rsx! {
        CardPanel {
            title: "渠道",
            hint: "URL + Key 是凭证容器；调度模型由候补池加入，名字不可改",
            count: channels.read().len(),
            open: open,
            on_toggle: on_toggle,

            // 渠道选择行
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
                // 三输入：名称 + URL + Key
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
                    div { class: "grid grid-cols-1 gap-3 lg:grid-cols-2",
                        // 左：候补池（临时，未进拓扑）
                        div { class: "flex min-h-0 flex-col gap-2 rounded-lg border border-dashed border-zinc-700 bg-zinc-950/60 p-3",
                            div { class: "flex items-center justify-between gap-2",
                                div {
                                    p { class: "text-xs text-zinc-300", "候补池" }
                                    p { class: "text-[11px] text-zinc-600", "拉取结果，尚未进入拓扑" }
                                }
                                button {
                                    class: "rounded border border-zinc-700 px-2 py-0.5 text-[11px] text-zinc-300 hover:border-zinc-500",
                                    onclick: move |_| {
                                        // mock 拉取：补几个上游模型名
                                        let mut w = channels.write();
                                        let have: Vec<String> = w[idx].dispatch.clone();
                                        for m in ["gpt-4o", "gpt-4o-mini", "gpt-5", "o3", "o3-mini"] {
                                            let exists = w[idx].candidates.iter().any(|(n, _)| n == m)
                                                || have.iter().any(|n| n == m);
                                            if !exists {
                                                w[idx].candidates.push((m.to_string(), false));
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
                                            let picked: Vec<String> = w[idx].candidates.iter()
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

                        // 右：调度模型（已进拓扑，不可改名）
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

// ============ 共享组件 ============

/// 可折叠卡片：标题行常驻，展开后是录入区 + 节点区（children）。
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
                span { class: "text-zinc-500", if open { "▾" } else { "▸" } }
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

/// 节点内容区：卡片下半部，装该实体的节点列表。
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

/// 实体芯片：点本体载入录入行编辑，点 ✕ 删除。
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

/// 双向绑定输入（配 Signal）。
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

/// 受控输入（配回调，用于写进结构体字段）。
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
