//! tavern-page-lorebook — 世界书 (Lorebook / World Info) 管理。
//!
//! 对齐 SillyTavern `#WorldInfo`:
//! - 世界书卡牌网格 (Web 5 栏 / 平板 3 栏 / 手机 1 栏)
//! - 每本世界书: 书名、词条数、常驻/动态策略、最后修改
//! - 条目编辑弹窗: Web/平板 3 栏分区 (关键词/触发设置 | 插入深度/策略 | 设定正文)，手机 1 栏堆叠

use dioxus::prelude::*;
use tavern_ui::{Dialog, EmptyState, Field};

/// 世界书条目
#[derive(Clone, PartialEq)]
pub struct LoreEntry {
    pub id: usize,
    pub title: String,
    pub keys: Vec<String>,
    pub content: String,
    pub is_constant: bool,
    pub depth: i32,
}

/// 世界书单本
#[derive(Clone, PartialEq)]
pub struct Lorebook {
    pub id: usize,
    pub name: String,
    pub description: String,
    pub entries_count: usize,
    pub token_budget: usize,
    pub entries: Vec<LoreEntry>,
    pub updated_at: String,
}

fn seed_lorebooks() -> Vec<Lorebook> {
    vec![
        Lorebook {
            id: 1,
            name: "娱乐圈资本生态规则".into(),
            description: "当代全球影视投资生态、S级项目主控权协议、艺人合规与保密条款。".into(),
            entries_count: 8,
            token_budget: 1200,
            entries: vec![
                LoreEntry {
                    id: 1,
                    title: "独立制片人豁免协议".into(),
                    keys: vec!["豁免协议".into(), "独立制片".into(), "对赌筹码".into()],
                    content: "【条款】豁免协议签署后，资方放弃对项目最终剪辑权的干涉，由制作人承担无限连带对赌。".into(),
                    is_constant: false,
                    depth: 4,
                },
                LoreEntry {
                    id: 2,
                    title: "正午阳光资方背景".into(),
                    keys: vec!["正午阳光".into(), "董事会".into()],
                    content: "【机构档案】头部内容制作方，对剧本严谨度与主创团队口碑有近乎苛刻的准入门槛。".into(),
                    is_constant: true,
                    depth: 2,
                },
            ],
            updated_at: "刚刚".into(),
        },
        Lorebook {
            id: 2,
            name: "全知读者世界线词典".into(),
            description: "星座名录、神圣星流直播系统法则、绝对化身契约与东湖大桥场景全览。".into(),
            entries_count: 14,
            token_budget: 2000,
            entries: vec![
                LoreEntry {
                    id: 1,
                    title: "灭活法世界观".into(),
                    keys: vec!["灭活法".into(), "连载".into(), "最终读者".into()],
                    content: "《在灭亡的世界中存活的三种方法》，仅有金独子一人读完全文的长篇小说。".into(),
                    is_constant: true,
                    depth: 1,
                },
            ],
            updated_at: "昨天".into(),
        },
        Lorebook {
            id: 3,
            name: "2069 械梦神经中枢规则".into(),
            description: "地底都市仿生人觉醒判定、水晶球记忆回廊、人类庇护协议。".into(),
            entries_count: 6,
            token_budget: 800,
            entries: vec![],
            updated_at: "3天前".into(),
        },
    ]
}

#[component]
pub fn LorebookPage() -> Element {
    let mut books = use_signal(seed_lorebooks);
    let mut editing = use_signal(|| None::<Lorebook>);
    let mut delete_id = use_signal(|| None::<usize>);

    rsx! {
        div { class: "relative flex h-full w-full flex-col gap-4 overflow-hidden",
            div { class: "flex shrink-0 items-center justify-between px-1",
                div { class: "flex flex-col gap-0.5",
                    h1 { class: "font-serif text-base font-bold text-zinc-100", "世界书 (Lorebook)" }
                    span { class: "text-xs text-zinc-500", "设定集与动态触发词典，只有提到关键词时才会动态注入上下文，节省 Token" }
                }
                button {
                    class: "flex items-center gap-1.5 rounded-full bg-gradient-to-r from-purple-600 to-pink-600 px-4 py-1.5 text-xs font-bold text-white shadow-lg shadow-purple-600/30 transition-all hover:scale-105",
                    onclick: move |_| {
                        let next_id = books().iter().map(|b| b.id).max().unwrap_or(0) + 1;
                        editing.set(Some(Lorebook {
                            id: next_id,
                            name: String::new(),
                            description: String::new(),
                            entries_count: 0,
                            token_budget: 1000,
                            entries: Vec::new(),
                            updated_at: "刚刚".into(),
                        }));
                    },
                    "+ 新建世界书"
                }
            }

            // 世界书卡牌网格: Web 5 栏 / 平板 3 栏 / 手机 1 栏
            div { class: "min-h-0 flex-1 overflow-y-auto px-1 pb-6",
                if books().is_empty() {
                    EmptyState {
                        title: "暂无世界书设定".to_string(),
                        hint: "为你的剧本添加专属设定集".to_string(),
                    }
                } else {
                    div { class: "grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3 xl:grid-cols-5",
                        for b in books().iter() {
                            {
                                let b_edit = b.clone();
                                let b_id = b.id;
                                rsx! {
                                    div {
                                        key: "{b.id}",
                                        class: "group relative flex cursor-pointer flex-col justify-between rounded-2xl border border-zinc-800/80 bg-zinc-900/60 p-4 transition-all duration-300 hover:-translate-y-0.5 hover:border-purple-500/50 hover:bg-zinc-900 hover:shadow-lg",
                                        onclick: move |_| editing.set(Some(b_edit.clone())),
                                        div { class: "flex items-center justify-between",
                                            span { class: "rounded-md bg-purple-500/10 border border-purple-500/30 px-2 py-0.5 text-[9px] font-bold text-purple-300",
                                                "📖 LOREBOOK"
                                            }
                                            span { class: "text-[10px] text-zinc-500", "{b.updated_at}" }
                                        }

                                        div { class: "flex flex-col gap-1.5 py-3",
                                            h3 { class: "font-serif text-sm font-bold text-zinc-100 group-hover:text-purple-300 transition-colors line-clamp-1",
                                                "{b.name}"
                                            }
                                            p { class: "line-clamp-2 text-xs leading-4 text-zinc-400 min-h-8",
                                                if b.description.is_empty() { "暂无设定概述…" } else { "{b.description}" }
                                            }
                                        }

                                        div { class: "flex items-center justify-between border-t border-zinc-800/60 pt-2 text-[10px] text-zinc-500",
                                            span { "{b.entries_count} 条目 · 约 {b.token_budget} tok" }
                                            button {
                                                class: "hover:text-rose-400 transition-colors",
                                                onclick: move |e| {
                                                    e.stop_propagation();
                                                    delete_id.set(Some(b_id));
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

            // 编辑弹窗
            if let Some(target) = editing() {
                div {
                    class: "fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-xl p-4",
                    onclick: move |_| editing.set(None),
                    div {
                        class: "relative flex h-full max-h-[720px] w-full max-w-2xl flex-col overflow-hidden rounded-3xl border border-zinc-800 bg-zinc-900 shadow-2xl p-6",
                        onclick: move |e| e.stop_propagation(),
                        LorebookEditor {
                            initial: target.clone(),
                            on_close: move |_| editing.set(None),
                            on_save: move |saved: Lorebook| {
                                let mut list = books.write();
                                if let Some(idx) = list.iter().position(|x| x.id == saved.id) {
                                    list[idx] = saved;
                                } else {
                                    list.insert(0, saved);
                                }
                                editing.set(None);
                            },
                        }
                    }
                }
            }

            Dialog {
                title: "删除世界书".to_string(),
                open: delete_id().is_some(),
                on_cancel: move |_| delete_id.set(None),
                on_confirm: move |_| {
                    if let Some(id) = delete_id() {
                        books.write().retain(|b| b.id != id);
                        delete_id.set(None);
                    }
                },
                "删除后，已绑定此世界书的剧本将不再触发其中的关键词注入。"
            }
        }
    }
}

/// 世界书编辑器: 表单 3 栏分区 (web/平板), 手机 1 栏堆叠
#[component]
fn LorebookEditor(
    initial: Lorebook,
    on_close: EventHandler<()>,
    on_save: EventHandler<Lorebook>,
) -> Element {
    let mut name = use_signal(|| initial.name.clone());
    let mut description = use_signal(|| initial.description.clone());
    let mut token_budget = use_signal(|| initial.token_budget);
    let mut new_key = use_signal(String::new);
    let mut sample_entry = use_signal(|| {
        initial
            .entries
            .first()
            .map(|e| e.content.clone())
            .unwrap_or_default()
    });

    let can_save = !name().trim().is_empty();

    rsx! {
        div { class: "flex h-full flex-col gap-4 text-xs text-zinc-100",
            div { class: "flex items-center justify-between border-b border-zinc-800 pb-3 select-none",
                span { class: "font-serif text-sm font-bold text-white",
                    if initial.name.is_empty() { "新建世界书" } else { "编辑世界书" }
                }
                button {
                    class: "text-zinc-400 hover:text-white",
                    onclick: move |_| on_close.call(()),
                    "✕"
                }
            }

            // 编辑菜单: Web/平板 3 栏，手机 1 栏
            div { class: "grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-y-auto pr-1 md:grid-cols-3",
                // 左侧 2 栏: 基础与触发词
                div { class: "flex flex-col gap-3 md:col-span-2",
                    Field { label: "世界书名称 *",
                        input {
                            class: "w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-xs text-zinc-100 outline-none focus:border-purple-500",
                            placeholder: "如：娱乐圈资本生态规则",
                            value: "{name()}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    Field { label: "概述与用途",
                        textarea {
                            class: "h-20 w-full resize-none rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-xs leading-5 text-zinc-100 outline-none focus:border-purple-500",
                            placeholder: "概述该世界书包含的内容类型…",
                            value: "{description()}",
                            oninput: move |e| description.set(e.value()),
                        }
                    }
                    Field { label: "核心触发条目内容 (示例)",
                        textarea {
                            class: "h-28 w-full resize-none rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-2 text-xs leading-5 text-zinc-100 outline-none focus:border-purple-500 font-mono",
                            placeholder: "输入条目设定内容，当检测到关键词时将完整送入提示词上下文…",
                            value: "{sample_entry()}",
                            oninput: move |e| sample_entry.set(e.value()),
                        }
                    }
                }

                // 右侧 1 栏: 触发配置与预算 (手机下排在下方)
                div { class: "flex flex-col gap-3 rounded-2xl border border-zinc-800/80 bg-zinc-950/60 p-3.5 md:col-span-1",
                    span { class: "font-semibold text-zinc-300", "触发与注入规则" }
                    Field { label: "Token 预算上限",
                        input {
                            r#type: "number",
                            class: "w-full rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-1.5 text-xs text-zinc-100 outline-none focus:border-purple-500",
                            value: "{token_budget()}",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse() {
                                    token_budget.set(v);
                                }
                            },
                        }
                    }
                    Field { label: "快速添加触发词",
                        div { class: "flex items-center gap-1.5",
                            input {
                                class: "w-full rounded-xl border border-zinc-800 bg-zinc-900 px-2.5 py-1 text-xs text-zinc-100 outline-none focus:border-purple-500",
                                placeholder: "如：对赌协议",
                                value: "{new_key()}",
                                oninput: move |e| new_key.set(e.value()),
                            }
                            button {
                                class: "rounded-xl bg-zinc-800 px-2.5 py-1 text-xs text-zinc-300 hover:bg-zinc-700",
                                onclick: move |_| new_key.set(String::new()),
                                "+"
                            }
                        }
                    }
                    div { class: "rounded-xl bg-zinc-900/60 p-2 text-[10px] text-zinc-500 leading-4",
                        "条目会在大模型推理前由文本扫描器命中触发词后动态挂载。"
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
                        let mut entries = initial.entries.clone();
                        if !sample_entry().is_empty() {
                            entries.push(LoreEntry {
                                id: entries.len() + 1,
                                title: "核心词条".into(),
                                keys: vec!["自动匹配".into()],
                                content: sample_entry(),
                                is_constant: false,
                                depth: 3,
                            });
                        }
                        on_save.call(Lorebook {
                            id: initial.id,
                            name: name(),
                            description: description(),
                            entries_count: entries.len(),
                            token_budget: token_budget(),
                            entries,
                            updated_at: "刚刚".into(),
                        });
                    },
                    "保存世界书"
                }
            }
        }
    }
}
