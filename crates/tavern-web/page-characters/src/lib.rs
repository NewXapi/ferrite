//! tavern-page-characters — 角色列表与角色卡编辑页。
//!
//! 当前为壳内效果页：角色数据放在本地信号，新建 / 编辑 / 删除在内存中生效；
//! 待 client/state 会话接线后换成 `/tavern/characters` 读写。

use dioxus::prelude::*;

/// 角色卡可编辑字段，对齐 `tavern-api/characters` 的角色 JSON。
#[derive(Clone, PartialEq)]
pub struct CharacterDraft {
    pub name: String,
    pub description: String,
    pub personality: String,
    pub scenario: String,
    pub first_mes: String,
    pub mes_example: String,
}

impl CharacterDraft {
    fn empty() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            personality: String::new(),
            scenario: String::new(),
            first_mes: String::new(),
            mes_example: String::new(),
        }
    }
}

/// 编辑器目标：`None` 新建，`Some(i)` 编辑列表第 i 项。
#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorTarget {
    New,
    Index(usize),
}

#[component]
pub fn CharactersPage() -> Element {
    let mut characters = use_signal(Vec::<CharacterDraft>::new);
    let mut editor = use_signal(|| None::<EditorTarget>);

    rsx! {
        if let Some(target) = editor() {
            CharacterEditor {
                initial: match target {
                    EditorTarget::New => CharacterDraft::empty(),
                    EditorTarget::Index(i) => characters()[i].clone(),
                },
                on_save: move |draft: CharacterDraft| {
                    match target {
                        EditorTarget::New => characters.write().push(draft),
                        EditorTarget::Index(i) => characters.write()[i] = draft,
                    }
                    editor.set(None);
                },
                on_cancel: move |_| editor.set(None),
            }
        } else {
            div { class: "flex h-full flex-col gap-4",
                div { class: "flex items-center justify-between",
                    span { class: "text-sm text-zinc-400", "共 {characters().len()} 个角色" }
                    button {
                        class: "rounded-full bg-zinc-100 px-3 py-1.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-300",
                        onclick: move |_| editor.set(Some(EditorTarget::New)),
                        "新建角色"
                    }
                }
                if characters().is_empty() {
                    div { class: "flex flex-1 flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-zinc-800 py-16 text-center",
                        span { class: "text-sm font-medium text-zinc-300", "还没有角色" }
                        span { class: "text-xs text-zinc-500", "点右上角「新建角色」创建第一张角色卡" }
                    }
                } else {
                    div { class: "grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3",
                        for (i, c) in characters().iter().enumerate() {
                            CharacterCard {
                                key: "{i}-{c.name}",
                                name: c.name.clone(),
                                summary: c.description.clone(),
                                on_edit: move |_| editor.set(Some(EditorTarget::Index(i))),
                                on_delete: move |_| {
                                    characters.write().remove(i);
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CharacterCard(
    name: String,
    summary: String,
    on_edit: EventHandler<MouseEvent>,
    on_delete: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "flex flex-col gap-2 rounded-xl border border-zinc-800 bg-zinc-900/60 p-4",
            div { class: "flex items-center gap-3",
                div { class: "flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-zinc-800 text-sm font-semibold text-zinc-300",
                    "{name.chars().next().unwrap_or('?')}"
                }
                span { class: "truncate text-sm font-medium text-zinc-100", "{name}" }
            }
            p { class: "line-clamp-3 min-h-6 text-xs leading-5 text-zinc-400",
                if summary.is_empty() { "（无描述）" } else { "{summary}" }
            }
            div { class: "mt-auto flex items-center justify-end gap-2 pt-1",
                button {
                    class: "rounded-full px-3 py-1 text-xs text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100",
                    onclick: on_edit,
                    "编辑"
                }
                button {
                    class: "rounded-full px-3 py-1 text-xs text-red-400/90 transition-colors hover:bg-red-950/60",
                    onclick: on_delete,
                    "删除"
                }
            }
        }
    }
}

#[component]
fn CharacterEditor(
    initial: CharacterDraft,
    on_save: EventHandler<CharacterDraft>,
    on_cancel: EventHandler<MouseEvent>,
) -> Element {
    let mut name = use_signal(|| initial.name.clone());
    let mut description = use_signal(|| initial.description.clone());
    let mut personality = use_signal(|| initial.personality.clone());
    let mut scenario = use_signal(|| initial.scenario.clone());
    let mut first_mes = use_signal(|| initial.first_mes.clone());
    let mut mes_example = use_signal(|| initial.mes_example.clone());

    let can_save = !name().trim().is_empty();

    rsx! {
        div { class: "flex h-full flex-col gap-4",
            div { class: "flex items-center justify-between",
                span { class: "text-sm font-medium text-zinc-200", "角色卡" }
                div { class: "flex items-center gap-2",
                    button {
                        class: "rounded-full px-3 py-1.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100",
                        onclick: on_cancel,
                        "取消"
                    }
                    button {
                        class: "rounded-full bg-zinc-100 px-3 py-1.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-300 disabled:cursor-not-allowed disabled:opacity-40",
                        disabled: !can_save,
                        onclick: move |_| {
                            on_save.call(CharacterDraft {
                                name: name(),
                                description: description(),
                                personality: personality(),
                                scenario: scenario(),
                                first_mes: first_mes(),
                                mes_example: mes_example(),
                            });
                        },
                        "保存"
                    }
                }
            }
            div { class: "grid min-h-0 flex-1 grid-cols-1 gap-3 overflow-y-auto lg:grid-cols-2",
                Field { label: "名字",
                    input {
                        class: "w-full rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-sm text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-zinc-600",
                        placeholder: "角色名",
                        value: "{name()}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                Field { label: "场景 scenario",
                    textarea {
                        class: "h-20 w-full resize-none rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-sm text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-zinc-600",
                        placeholder: "对话发生的场景",
                        value: "{scenario()}",
                        oninput: move |e| scenario.set(e.value()),
                    }
                }
                Field { label: "描述 description",
                    textarea {
                        class: "h-28 w-full resize-none rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-sm text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-zinc-600",
                        placeholder: "角色背景、外貌、经历",
                        value: "{description()}",
                        oninput: move |e| description.set(e.value()),
                    }
                }
                Field { label: "性格 personality",
                    textarea {
                        class: "h-28 w-full resize-none rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-sm text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-zinc-600",
                        placeholder: "性格摘要",
                        value: "{personality()}",
                        oninput: move |e| personality.set(e.value()),
                    }
                }
                Field { label: "开场白 first_mes",
                    textarea {
                        class: "h-28 w-full resize-none rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-sm text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-zinc-600",
                        placeholder: "角色的第一条消息",
                        value: "{first_mes()}",
                        oninput: move |e| first_mes.set(e.value()),
                    }
                }
                Field { label: "对话样例 mes_example",
                    textarea {
                        class: "h-28 w-full resize-none rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-sm text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-zinc-600",
                        placeholder: "<START> 分隔的样例对话",
                        value: "{mes_example()}",
                        oninput: move |e| mes_example.set(e.value()),
                    }
                }
            }
        }
    }
}

#[component]
fn Field(label: &'static str, children: Element) -> Element {
    rsx! {
        label { class: "flex flex-col gap-1.5",
            span { class: "text-xs font-medium text-zinc-400", "{label}" }
            {children}
        }
    }
}
