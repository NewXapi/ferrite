//! 管理面板的三层实体编辑页：分组（顶层）→ 模型映射（中层）→ 渠道模型（底层）。
//! 每层布局一致：顶部实体 chip 行 → 中间操作按钮行 → 底部三列编辑区。
//! 层切换由 home.rs 的面板顶部 tab 负责，本文件只提供各层内容。
//! 数据 mock，灰度风格。

use dioxus::prelude::*;

// ============ 第一层：分组 ============

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
    let description = use_signal(|| "所有新用户的默认归属".to_string());
    let mut enabled = use_signal(|| true);
    let channels_ref = use_signal(|| "OpenAI 官方\nOneAPI 上游\nAzure East".to_string());
    let models_ref = use_signal(|| "gpt-4o\ngpt-5\ngemini-2.5-pro".to_string());

    rsx! {
        LayerShell {
            chips: rsx! {
                ChipRow {
                    items: GROUPS,
                    active: active.cloned(),
                    onselect: move |v: String| active.set(v),
                }
            },
            controls: rsx! {
                ControlRow {
                    actions: vec![
                        ("新建分组", ButtonTone::Default),
                        ("重命名", ButtonTone::Default),
                        ("启用", ButtonTone::Default),
                        ("禁用", ButtonTone::Default),
                        ("引用检查", ButtonTone::Default),
                        ("删除", ButtonTone::Danger),
                        ("保存", ButtonTone::Primary),
                    ],
                }
            },
            Pane { title: "基础",
                Field { label: "分组名", value: name, placeholder: "default" }
                Field { label: "展示名", value: display_name, placeholder: "默认分组" }
                Toggle { label: "启用", on: enabled(), onclick: move |_| enabled.set(!enabled()) }
            }
            Pane { title: "说明",
                Textarea { label: "描述", value: description, placeholder: "这个分组的用途…" }
            }
            Pane { title: "引用（只读）",
                Textarea { label: "关联渠道", value: channels_ref, placeholder: "—" }
                Textarea { label: "可用模型（派生）", value: models_ref, placeholder: "—" }
            }
        }
    }
}

// ============ 第二层：模型映射 ============

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
    let description = use_signal(|| "OpenAI 旗舰多模态模型".to_string());
    let cover = use_signal(|| "https://cdn.example.com/openai.svg".to_string());
    let badge = use_signal(|| "推荐".to_string());
    let tags = use_signal(|| "多模态, 长上下文".to_string());
    let mut visible = use_signal(|| true);
    let providers = use_signal(|| {
        "OpenAI 官方 → gpt-4o\nAzure East → gpt-4o\nOneAPI 上游 → gpt-4o".to_string()
    });

    rsx! {
        LayerShell {
            chips: rsx! {
                ChipRow {
                    items: MAPPINGS,
                    active: active.cloned(),
                    onselect: move |v: String| active.set(v),
                }
            },
            controls: rsx! {
                ControlRow {
                    actions: vec![
                        ("显示", ButtonTone::Default),
                        ("隐藏", ButtonTone::Default),
                        ("上移", ButtonTone::Default),
                        ("下移", ButtonTone::Default),
                        ("清理孤儿", ButtonTone::Default),
                        ("保存", ButtonTone::Primary),
                    ],
                }
            },
            Pane { title: "标识",
                Field { label: "别名（只读，来自渠道）", value: alias, placeholder: "gpt-4o" }
                Field { label: "展示名", value: display_name, placeholder: "GPT-4o" }
                Toggle { label: "对用户可见", on: visible(), onclick: move |_| visible.set(!visible()) }
            }
            Pane { title: "展示",
                Field { label: "封面 URL", value: cover, placeholder: "https://…" }
                Field { label: "角标", value: badge, placeholder: "new / beta / 推荐" }
                Field { label: "标签（逗号分隔）", value: tags, placeholder: "多模态, 长上下文" }
            }
            Pane { title: "描述与来源",
                Textarea { label: "描述", value: description, placeholder: "这个模型的说明…" }
                Textarea { label: "提供方（只读）", value: providers, placeholder: "—" }
            }
        }
    }
}

// ============ 第三层：渠道模型 ============

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
    let channel_type = use_signal(|| "openai".to_string());
    let remark = use_signal(|| "主力渠道".to_string());
    let base_urls = use_signal(|| "https://api.openai.com/v1".to_string());
    let api_keys = use_signal(|| "sk-**************************".to_string());
    let groups = use_signal(|| "default, vip".to_string());
    let models = use_signal(|| {
        "gpt-4o -> gpt-4o-2024-11-20\ngpt-5 -> gpt-5\ngpt-4o-mini -> gpt-4o-mini".to_string()
    });
    let mut enabled = use_signal(|| true);

    rsx! {
        LayerShell {
            chips: rsx! {
                ChipRow {
                    items: CHANNELS,
                    active: active.cloned(),
                    onselect: move |v: String| active.set(v),
                }
            },
            controls: rsx! {
                ControlRow {
                    actions: vec![
                        ("新建渠道", ButtonTone::Default),
                        ("复制", ButtonTone::Default),
                        ("启用", ButtonTone::Default),
                        ("禁用", ButtonTone::Default),
                        ("测试连通", ButtonTone::Default),
                        ("拉取模型", ButtonTone::Default),
                        ("删除", ButtonTone::Danger),
                        ("保存", ButtonTone::Primary),
                    ],
                }
            },
            Pane { title: "基础",
                Field { label: "渠道名称", value: name, placeholder: "OpenAI 官方" }
                Field { label: "渠道类型", value: channel_type, placeholder: "openai" }
                Field { label: "备注", value: remark, placeholder: "可选" }
                Toggle { label: "启用", on: enabled(), onclick: move |_| enabled.set(!enabled()) }
            }
            Pane { title: "端点与认证",
                Textarea { label: "Base URL（一行一个，首行为主）", value: base_urls, placeholder: "https://…" }
                Textarea { label: "API Key（一行一个）", value: api_keys, placeholder: "sk-…" }
            }
            Pane { title: "路由",
                Field { label: "分组（逗号分隔）", value: groups, placeholder: "default, vip" }
                Textarea { label: "模型（alias -> upstream，一行一个）", value: models, placeholder: "gpt-4o -> gpt-4o-2024-11-20" }
            }
        }
    }
}

// ============ 共享子组件 ============

/// 每层的统一骨架：chip 行 → 操作行 → 三列编辑区（children 即三个 Pane）。
#[component]
fn LayerShell(chips: Element, controls: Element, children: Element) -> Element {
    rsx! {
        div { class: "flex h-full min-h-0 flex-col gap-4",
            {chips}
            {controls}
            section { class: "grid min-h-0 flex-1 grid-cols-1 gap-4 lg:grid-cols-3",
                {children}
            }
        }
    }
}

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

#[component]
fn Toggle(label: &'static str, on: bool, onclick: EventHandler<MouseEvent>) -> Element {
    let knob = if on { "translate-x-4" } else { "translate-x-0" };
    let track = if on { "bg-zinc-100" } else { "bg-zinc-800" };
    rsx! {
        label { class: "flex shrink-0 items-center justify-between gap-3 rounded-md border border-zinc-800 bg-zinc-950 px-3 py-2",
            span { class: "text-xs text-zinc-400", "{label}" }
            button {
                r#type: "button",
                class: "relative h-5 w-9 rounded-full transition-colors {track}",
                onclick: move |event| onclick.call(event),
                span { class: "absolute left-0.5 top-0.5 h-4 w-4 rounded-full bg-zinc-900 transition-transform {knob}" }
            }
        }
    }
}
