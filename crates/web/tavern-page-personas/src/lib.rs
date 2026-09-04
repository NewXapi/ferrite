//! tavern-page-personas — 用户人格管理。
//!
//! 对齐 SillyTavern Persona Management:
//! - 人格卡牌网格 (Web 5 栏 / 平板 3 栏 / 手机 1 栏)
//! - 每个人格: 头像、名字、身份描述、绑定剧本数
//! - 编辑弹窗: 头像占位、名字、描述 (表单 3 栏分区 web/平板, 1 栏手机)

use dioxus::prelude::*;
use tavern_ui::{Avatar, Dialog, EmptyState, Field};

/// 用户人格
#[derive(Clone, PartialEq)]
pub struct Persona {
    pub id: usize,
    pub name: String,
    pub title: String,
    pub description: String,
    pub avatar_src: Option<String>,
    pub bound_scripts: usize,
    pub is_default: bool,
}

fn seed_personas() -> Vec<Persona> {
    vec![
        Persona {
            id: 1,
            name: "夜阑".into(),
            title: "独立制片人".into(),
            description: "三十五岁，雷厉风行。信奉资本效率至上，谈判桌上从不先亮底牌。".into(),
            avatar_src: None,
            bound_scripts: 3,
            is_default: true,
        },
        Persona {
            id: 2,
            name: "阿澈".into(),
            title: "见习练习生".into(),
            description: "十九岁，刚进公司的新人。怀揣舞台梦想，对娱乐圈规则一无所知。".into(),
            avatar_src: None,
            bound_scripts: 1,
            is_default: false,
        },
        Persona {
            id: 3,
            name: "老 K".into(),
            title: "资深娱记".into(),
            description: "混迹名利场二十年的老油子，手里攥着半个圈子的黑料。".into(),
            avatar_src: None,
            bound_scripts: 0,
            is_default: false,
        },
    ]
}

#[component]
pub fn PersonasPage() -> Element {
    let mut personas = use_signal(seed_personas);
    let mut editing = use_signal(|| None::<Persona>);
    let mut delete_id = use_signal(|| None::<usize>);

    rsx! {
        div { class: "relative flex h-full w-full flex-col gap-4 overflow-hidden",
            // 顶栏
            div { class: "flex shrink-0 items-center justify-between px-1",
                div { class: "flex flex-col gap-0.5",
                    h1 { class: "font-serif text-base font-bold text-zinc-100", "我的人格" }
                    span { class: "text-xs text-zinc-500", "在进入剧本前选择你扮演的身份；人格设定会注入上下文" }
                }
                button {
                    class: "flex items-center gap-1.5 rounded-full bg-gradient-to-r from-purple-600 to-pink-600 px-4 py-1.5 text-xs font-bold text-white shadow-lg shadow-purple-600/30 transition-all hover:scale-105",
                    onclick: move |_| {
                        let next_id = personas().iter().map(|p| p.id).max().unwrap_or(0) + 1;
                        editing.set(Some(Persona {
                            id: next_id,
                            name: String::new(),
                            title: String::new(),
                            description: String::new(),
                            avatar_src: None,
                            bound_scripts: 0,
                            is_default: false,
                        }));
                    },
                    "+ 新建人格"
                }
            }

            // 人格卡牌网格: Web 5 栏 / 平板 3 栏 / 手机 1 栏
            div { class: "min-h-0 flex-1 overflow-y-auto px-1 pb-6",
                if personas().is_empty() {
                    EmptyState {
                        title: "还没有人格".to_string(),
                        hint: "创建你在故事中的化身".to_string(),
                    }
                } else {
                    div { class: "grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 xl:grid-cols-5",
                        for p in personas().iter() {
                            {
                                let p_edit = p.clone();
                                let p_id = p.id;
                                rsx! {
                                    div {
                                        key: "{p.id}",
                                        class: "group relative flex cursor-pointer flex-col items-center gap-2.5 rounded-2xl border border-zinc-800/80 bg-zinc-900/60 p-4 text-center transition-all duration-300 hover:-translate-y-0.5 hover:border-purple-500/50 hover:shadow-lg hover:shadow-purple-950/20",
                                        onclick: move |_| editing.set(Some(p_edit.clone())),
                                        if p.is_default {
                                            span { class: "absolute right-2 top-2 rounded-full bg-purple-500/20 border border-purple-500/40 px-2 py-0.5 text-[9px] font-bold text-purple-300",
                                                "默认"
                                            }
                                        }
                                        Avatar { name: p.name.clone(), src: p.avatar_src.clone(), size: "h-16 w-16 text-xl".to_string() }
                                        div { class: "flex flex-col gap-0.5",
                                            span { class: "text-xs font-bold text-zinc-100", "{p.name}" }
                                            span { class: "text-[10px] text-purple-300", "{p.title}" }
                                        }
                                        p { class: "line-clamp-2 min-h-8 text-[11px] leading-4 text-zinc-400",
                                            if p.description.is_empty() { "（暂无描述）" } else { "{p.description}" }
                                        }
                                        div { class: "flex items-center justify-between w-full border-t border-zinc-800/60 pt-2 text-[10px] text-zinc-500",
                                            span { "绑定 {p.bound_scripts} 部剧本" }
                                            button {
                                                class: "text-zinc-500 hover:text-rose-400 transition-colors",
                                                onclick: move |e| {
                                                    e.stop_propagation();
                                                    delete_id.set(Some(p_id));
                                                },
                                                "删除"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 编辑弹窗 (背景景深模糊)
            if let Some(target) = editing() {
                div {
                    class: "fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-xl p-4",
                    onclick: move |_| editing.set(None),
                    div {
                        class: "relative flex w-full max-w-md flex-col gap-4 rounded-3xl border border-zinc-800 bg-zinc-900 p-6 shadow-2xl",
                        onclick: move |e| e.stop_propagation(),
                        PersonaEditor {
                            initial: target.clone(),
                            on_close: move |_| editing.set(None),
                            on_save: move |saved: Persona| {
                                let mut list = personas.write();
                                if let Some(idx) = list.iter().position(|x| x.id == saved.id) {
                                    list[idx] = saved;
                                } else {
                                    list.push(saved);
                                }
                                editing.set(None);
                            },
                        }
                    }
                }
            }

            Dialog {
                title: "删除人格".to_string(),
                open: delete_id().is_some(),
                on_cancel: move |_| delete_id.set(None),
                on_confirm: move |_| {
                    if let Some(id) = delete_id() {
                        personas.write().retain(|p| p.id != id);
                        delete_id.set(None);
                    }
                },
                "删除后，绑定此人格的剧本将回退到默认人格。"
            }
        }
    }
}

/// 人格编辑器: 表单 3 栏分区 (web/平板) / 1 栏堆叠 (手机)
#[component]
fn PersonaEditor(
    initial: Persona,
    on_close: EventHandler<()>,
    on_save: EventHandler<Persona>,
) -> Element {
    let mut name = use_signal(|| initial.name.clone());
    let mut title = use_signal(|| initial.title.clone());
    let mut description = use_signal(|| initial.description.clone());

    let can_save = !name().trim().is_empty();

    rsx! {
        div { class: "flex items-center justify-between border-b border-zinc-800 pb-3 select-none",
            span { class: "text-sm font-bold text-zinc-100",
                if initial.name.is_empty() { "新建人格" } else { "编辑人格" }
            }
            button {
                class: "text-zinc-400 hover:text-zinc-100",
                onclick: move |_| on_close.call(()),
                "✕"
            }
        }

        // 编辑菜单: web/平板 3 栏分区 (头像 | 名字/头衔 | 描述), 手机 1 栏堆叠
        div { class: "grid grid-cols-1 gap-4 md:grid-cols-3",
            div { class: "flex flex-col items-center gap-2 md:col-span-1",
                Avatar { name: name(), size: "h-20 w-20 text-2xl".to_string() }
                button { class: "rounded-lg border border-dashed border-zinc-700 px-3 py-1 text-[11px] text-zinc-400 hover:border-purple-500 hover:text-purple-300 transition-colors",
                    "上传头像"
                }
            }
            div { class: "flex flex-col gap-3 md:col-span-2",
                Field { label: "人格名 *",
                    input {
                        class: "w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-xs text-zinc-100 outline-none focus:border-purple-500",
                        placeholder: "如：夜阑",
                        value: "{name()}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                Field { label: "身份头衔",
                    input {
                        class: "w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-xs text-zinc-100 outline-none focus:border-purple-500",
                        placeholder: "如：独立制片人",
                        value: "{title()}",
                        oninput: move |e| title.set(e.value()),
                    }
                }
            }
            div { class: "md:col-span-3",
                Field { label: "人格描述 (注入上下文)",
                    textarea {
                        class: "h-24 w-full resize-none rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-xs leading-5 text-zinc-100 outline-none focus:border-purple-500",
                        placeholder: "你扮演谁？年龄、性格、行事风格、底线…",
                        value: "{description()}",
                        oninput: move |e| description.set(e.value()),
                    }
                }
            }
        }

        div { class: "flex items-center justify-end gap-2 border-t border-zinc-800 pt-3 select-none",
            button {
                class: "rounded-full px-4 py-1.5 text-xs text-zinc-400 hover:bg-zinc-800 transition-colors",
                onclick: move |_| on_close.call(()),
                "取消"
            }
            button {
                class: "rounded-full bg-gradient-to-r from-purple-600 to-pink-600 px-5 py-1.5 text-xs font-bold text-white shadow-md hover:scale-105 transition-all disabled:opacity-40",
                disabled: !can_save,
                onclick: move |_| {
                    on_save.call(Persona {
                        id: initial.id,
                        name: name(),
                        title: title(),
                        description: description(),
                        avatar_src: initial.avatar_src.clone(),
                        bound_scripts: initial.bound_scripts,
                        is_default: initial.is_default,
                    });
                },
                "保存人格"
            }
        }
    }
}
