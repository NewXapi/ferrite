//! 管理面板的三层实体编辑页：分组 / 模型映射 / 渠道模型。
//! 每层布局：顶部实体 chip 行 → 中间操作按钮行 → 底部编辑区。
//! 层切换由 home.rs 的面板顶部 tab 负责。
//! 数据 mock。

use dioxus::prelude::*;

// ============ 分组 ============

#[component]
pub fn GroupsLayer() -> Element {
    // (id, label, enabled)
    const GROUPS: &[(&str, &str, bool)] = &[
        ("default", "default", true),
        ("vip", "vip", true),
        ("claude", "claude", true),
        ("gpt-5", "gpt-5", true),
        ("internal", "internal", false),
        ("trial", "trial", true),
    ];

    let mut active = use_signal(|| "default".to_string());
    let name = use_signal(|| "default".to_string());
    let display_name = use_signal(|| "默认分组".to_string());

    rsx! {
        div { class: "flex h-full min-h-0 flex-col gap-4",
            ChipRow {
                items: GROUPS,
                active: active.cloned(),
                onselect: move |v: String| active.set(v),
            }
            ControlRow {
                actions: vec![
                    ("新建分组", ButtonTone::Default),
                    ("重命名", ButtonTone::Default),
                    ("删除", ButtonTone::Danger),
                    ("保存", ButtonTone::Primary),
                ],
            }
            // 分组只有名字与展示名，单列即可；留白比撑满三栏更诚实。
            section { class: "min-h-0 flex-1",
                div { class: "max-w-md",
                    Pane { title: "基础",
                        Field { label: "分组名", value: name, placeholder: "default" }
                        Field { label: "展示名", value: display_name, placeholder: "默认分组" }
                    }
                }
            }
        }
    }
}

// ============ 模型映射 ============

#[component]
pub fn MappingsLayer() -> Element {
    const MAPPINGS: &[(&str, &str, bool)] = &[
        ("gpt-4o", "gpt-4o", true),
        ("gpt-5", "gpt-5", true),
        ("claude-sonnet-4", "claude-sonnet-4", true),
        ("gemini-2.5-pro", "gemini-2.5-pro", true),
        ("o3-mini", "o3-mini", false),
    ];

    let mut active = use_signal(|| "gpt-4o".to_string());
    let alias = use_signal(|| "gpt-4o".to_string());
    let display_name = use_signal(|| "GPT-4o".to_string());

    rsx! {
        div { class: "flex h-full min-h-0 flex-col gap-4",
            ChipRow {
                items: MAPPINGS,
                active: active.cloned(),
                onselect: move |v: String| active.set(v),
            }
            ControlRow {
                actions: vec![
                    ("显示", ButtonTone::Default),
                    ("隐藏", ButtonTone::Default),
                    ("保存", ButtonTone::Primary),
                ],
            }
            section { class: "min-h-0 flex-1",
                div { class: "max-w-md space-y-3",
                    Pane { title: "标识",
                        Field { label: "别名（只读，来自渠道）", value: alias, placeholder: "gpt-4o" }
                        Field { label: "展示名", value: display_name, placeholder: "GPT-4o" }
                    }
                    p { class: "text-[11px] leading-relaxed text-zinc-600",
                        "卡牌样式（封面、角标、标签、描述）留待后续统一设计。"
                    }
                }
            }
        }
    }
}

// ============ 渠道模型 ============

#[component]
pub fn ChannelsLayer() -> Element {
    const CHANNELS: &[(&str, &str, bool)] = &[
        ("openai", "OpenAI 官方", true),
        ("azure", "Azure East", true),
        ("oneapi", "OneAPI 上游", true),
        ("anthropic", "Anthropic", false),
        ("bedrock", "AWS Bedrock", true),
        ("gemini", "Gemini", true),
        ("ollama", "Ollama 本地", false),
        ("custom1", "自建网关 A", true),
    ];

    let mut active = use_signal(|| "openai".to_string());
    let name = use_signal(|| "OpenAI 官方".to_string());
    let base_url = use_signal(|| "https://api.openai.com/v1".to_string());
    let api_keys = use_signal(|| "sk-**************************".to_string());

    rsx! {
        div { class: "flex h-full min-h-0 flex-col gap-4",
            ChipRow {
                items: CHANNELS,
                active: active.cloned(),
                onselect: move |v: String| active.set(v),
            }
            ControlRow {
                actions: vec![
                    ("新建渠道", ButtonTone::Default),
                    ("拉取模型", ButtonTone::Default),
                    ("删除", ButtonTone::Danger),
                    ("保存", ButtonTone::Primary),
                ],
            }
            // 名称 + URL + 多 key + 模型（由 key 拉取后勾选）
            section { class: "grid min-h-0 flex-1 grid-cols-1 gap-4 lg:grid-cols-2",
                Pane { title: "接入",
                    Field { label: "渠道名称", value: name, placeholder: "OpenAI 官方" }
                    Field { label: "Base URL", value: base_url, placeholder: "https://…" }
                    Textarea { label: "API Key（一行一个）", value: api_keys, placeholder: "sk-…" }
                }
                Pane { title: "模型",
                    ModelPicker {}
                }
            }
        }
    }
}

/// 模型列表：勾选「这个 key 能取到的模型」中要启用的项。
/// 「拉取模型」按钮填充候选，此处只做启用与否。
#[component]
fn ModelPicker() -> Element {
    // (model, enabled)
    let models = use_signal(|| {
        vec![
            ("gpt-4o".to_string(), true),
            ("gpt-4o-mini".to_string(), true),
            ("gpt-5".to_string(), true),
            ("o3".to_string(), false),
            ("o3-mini".to_string(), false),
            ("text-embedding-3-large".to_string(), false),
        ]
    });

    rsx! {
        div { class: "flex min-h-0 flex-1 flex-col gap-2",
            div { class: "flex items-center justify-between",
                span { class: "text-[11px] text-zinc-500", "已拉取 {models.read().len()} 个候选" }
                div { class: "flex gap-1.5",
                    button { class: "rounded border border-zinc-700 px-2 py-0.5 text-[11px] text-zinc-300 hover:border-zinc-500", "全选" }
                    button { class: "rounded border border-zinc-700 px-2 py-0.5 text-[11px] text-zinc-300 hover:border-zinc-500", "全不选" }
                }
            }
            div { class: "min-h-0 flex-1 space-y-1 overflow-y-auto rounded-md border border-zinc-800 bg-zinc-950 p-2",
                for (i, (model, on)) in models.read().iter().enumerate() {
                    {
                        let mut models = models;
                        let checked = *on;
                        let label = model.clone();
                        rsx! {
                            label { class: "flex cursor-pointer items-center gap-2 rounded px-2 py-1 hover:bg-zinc-900",
                                input {
                                    r#type: "checkbox",
                                    class: "accent-zinc-100",
                                    checked: checked,
                                    onchange: move |_| {
                                        let mut w = models.write();
                                        w[i].1 = !w[i].1;
                                    },
                                }
                                span { class: "font-mono text-xs text-zinc-300", "{label}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ============ 共享子组件 ============

#[derive(Clone, Copy, PartialEq, Eq)]
enum ButtonTone {
    Default,
    Primary,
    Danger,
}

/// 顶部实体 chip 行：点击切换当前编辑对象，圆点表示启用状态。
#[component]
fn ChipRow(
    items: &'static [(&'static str, &'static str, bool)],
    active: String,
    onselect: EventHandler<String>,
) -> Element {
    rsx! {
        section { class: "shrink-0 rounded-xl border border-zinc-800 bg-zinc-900/60 p-3",
            div { class: "flex flex-wrap items-center gap-2",
                for (id, label, ok) in items.iter() {
                    Chip {
                        id: id,
                        label: label,
                        ok: *ok,
                        active: active == *id,
                        onclick: move |v: String| onselect.call(v),
                    }
                }
            }
        }
    }
}

#[component]
fn Chip(
    id: &'static str,
    label: &'static str,
    ok: bool,
    active: bool,
    onclick: EventHandler<String>,
) -> Element {
    let tone = match (active, ok) {
        (true, _) => "border-zinc-100 bg-zinc-100 text-zinc-900",
        (false, true) => "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500",
        (false, false) => "border-zinc-800 bg-zinc-950 text-zinc-600 hover:border-zinc-700",
    };
    let dot = if ok { "bg-emerald-500" } else { "bg-zinc-600" };
    let id_owned = id.to_string();
    rsx! {
        button {
            class: "inline-flex items-center gap-2 rounded-full border px-3 py-1 text-xs font-medium transition-colors {tone}",
            onclick: move |_| onclick.call(id_owned.clone()),
            span { class: "h-1.5 w-1.5 rounded-full {dot}" }
            "{label}"
        }
    }
}

/// 中间操作按钮行，居中排列。
#[component]
fn ControlRow(actions: Vec<(&'static str, ButtonTone)>) -> Element {
    rsx! {
        section { class: "shrink-0 rounded-xl border border-zinc-800 bg-zinc-900/60 p-3",
            div { class: "flex flex-wrap items-center justify-center gap-2",
                for (label, tone) in actions.iter() {
                    ActionBtn { label: label, tone: *tone }
                }
            }
        }
    }
}

#[component]
fn ActionBtn(
    label: &'static str,
    #[props(default = ButtonTone::Default)] tone: ButtonTone,
) -> Element {
    let cls = match tone {
        ButtonTone::Primary => "border-zinc-100 bg-zinc-100 text-zinc-900 hover:bg-zinc-300",
        ButtonTone::Danger => {
            "border-zinc-800 text-zinc-400 hover:border-red-700 hover:text-red-400"
        }
        ButtonTone::Default => {
            "border-zinc-800 bg-zinc-900 text-zinc-300 hover:border-zinc-600 hover:text-zinc-100"
        }
    };
    rsx! {
        button {
            class: "rounded-md border px-3 py-1.5 text-xs font-medium transition-colors {cls}",
            "{label}"
        }
    }
}

#[component]
fn Pane(title: &'static str, children: Element) -> Element {
    rsx! {
        div { class: "flex min-h-0 flex-col gap-3 overflow-y-auto rounded-xl border border-zinc-800 bg-zinc-900/60 p-4",
            h3 { class: "shrink-0 text-xs font-medium uppercase tracking-wider text-zinc-500", "{title}" }
            div { class: "flex min-h-0 flex-1 flex-col gap-3", {children} }
        }
    }
}

#[component]
fn Field(label: &'static str, value: Signal<String>, placeholder: &'static str) -> Element {
    rsx! {
        label { class: "block shrink-0 space-y-1.5",
            span { class: "text-xs text-zinc-400", "{label}" }
            input {
                class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
                value: "{value.read()}",
                placeholder: "{placeholder}",
                oninput: move |event| value.set(event.value()),
            }
        }
    }
}

#[component]
fn Textarea(label: &'static str, value: Signal<String>, placeholder: &'static str) -> Element {
    rsx! {
        label { class: "flex min-h-0 flex-1 flex-col gap-1.5",
            span { class: "shrink-0 text-xs text-zinc-400", "{label}" }
            textarea {
                class: "min-h-[88px] w-full flex-1 resize-none rounded-md border border-zinc-800 bg-zinc-950 px-3 py-2 font-mono text-xs leading-relaxed text-zinc-200 outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
                value: "{value.read()}",
                placeholder: "{placeholder}",
                oninput: move |event| value.set(event.value()),
            }
        }
    }
}
