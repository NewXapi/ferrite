//! tavern-page-characters — 角色列表与卡片编辑。
//!
//! 对齐 SillyTavern `#right-nav-panel`:
//! - 顶部控制栏:搜索框 + 排序/筛选占位 +「新建角色」
//! - ST 式角色卡片网格:大头像、名字覆盖层、hover 浮现操作按钮(编辑、删除、聊天)
//! - 抽屉式编辑器(右侧滑出):头像预览、六核心字段、标签、取消/保存
//!
//! 当前为壳内效果页,内置两张示例角色卡展示排版,待 client/state 接线。

use dioxus::prelude::*;
use tavern_ui::{Avatar, Dialog, EmptyState, Field, IconButton};

/// 角色卡草稿,对齐 tavern-api characters JSON 形状。
#[derive(Clone, PartialEq)]
pub struct Character {
    pub id: usize,
    pub name: String,
    pub avatar_src: Option<String>,
    pub description: String,
    pub personality: String,
    pub scenario: String,
    pub first_mes: String,
    pub mes_example: String,
    pub tags: Vec<String>,
}

impl Character {
    fn empty(id: usize) -> Self {
        Self {
            id,
            name: String::new(),
            avatar_src: None,
            description: String::new(),
            personality: String::new(),
            scenario: String::new(),
            first_mes: String::new(),
            mes_example: String::new(),
            tags: Vec::new(),
        }
    }
}

fn seed_characters() -> Vec<Character> {
    vec![
        Character {
            id: 1,
            name: "巡音ルカ".into(),
            avatar_src: None,
            description: "粉色长发、冷艳沉静的歌手。声音沉稳有穿透力，私下话少但细心。".into(),
            personality: "冷静、温柔、略带神秘感、专业".into(),
            scenario: "录音室排练结束后的傍晚".into(),
            first_mes: "今天录音结束得比较早……你有空吗？我知道一家安静的咖啡馆。".into(),
            mes_example: "<START>\n{{user}}: 今天表现很棒。\n{{char}}: 谢谢，有你在台下，发挥更稳定了。".into(),
            tags: vec!["VOCALOID".into(), "歌手".into(), "温柔".into()],
        },
        Character {
            id: 2,
            name: "初音ミク".into(),
            avatar_src: None,
            description: "青葱色双马尾的元气电子歌姬。总是活力满满，对新事物充满好奇。".into(),
            personality: "活泼、元气、开朗、乐天".into(),
            scenario: "演播厅后台的休息时间".into(),
            first_mes: "呀吼～！下一场演出准备好一起狂欢了吗？".into(),
            mes_example: "<START>\n{{user}}: 准备好了！\n{{char}}: 那就跟紧我的节奏，出发咯～！".into(),
            tags: vec!["VOCALOID".into(), "元气".into(), "歌姬".into()],
        },
    ]
}

#[component]
pub fn CharactersPage() -> Element {
    let mut characters = use_signal(seed_characters);
    let mut search = use_signal(String::new);
    let mut editing = use_signal(|| None::<Character>);
    let mut delete_id = use_signal(|| None::<usize>);

    let query = search().to_lowercase();
    let filtered: Vec<Character> = characters
        .read()
        .clone()
        .into_iter()
        .filter(|c| {
            if query.is_empty() {
                true
            } else {
                c.name.to_lowercase().contains(&query)
                    || c.tags.iter().any(|t| t.to_lowercase().contains(&query))
                    || c.description.to_lowercase().contains(&query)
            }
        })
        .collect();

    rsx! {
        div { class: "relative flex h-full flex-col gap-4 overflow-hidden",
            // 顶部搜索 + 操作栏
            div { class: "flex shrink-0 items-center justify-between gap-3",
                div { class: "flex flex-1 items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900/60 px-3 py-1.5",
                    span { class: "text-xs text-zinc-500", "🔍" }
                    input {
                        class: "w-full bg-transparent text-xs text-zinc-200 outline-none placeholder:text-zinc-600",
                        placeholder: "搜索角色名、标签、描述…",
                        value: "{search()}",
                        oninput: move |e| search.set(e.value()),
                    }
                }
                button {
                    class: "flex shrink-0 items-center gap-1.5 rounded-full bg-zinc-100 px-3.5 py-1.5 text-xs font-semibold text-zinc-900 transition-colors hover:bg-zinc-300",
                    onclick: move |_| {
                        let next_id = characters().iter().map(|c| c.id).max().unwrap_or(0) + 1;
                        editing.set(Some(Character::empty(next_id)));
                    },
                    span { "+ 新建角色" }
                }
            }

            // 角色网格主视区
            div { class: "min-h-0 flex-1 overflow-y-auto",
                if filtered.is_empty() {
                    EmptyState {
                        title: "没有匹配的角色".to_string(),
                        hint: "尝试清除搜索关键词，或点击右上角新建角色".to_string(),
                    }
                } else {
                    div { class: "grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4",
                        for c in filtered {
                            CharacterCard {
                                key: "{c.id}",
                                char: c.clone(),
                                on_edit: {
                                    let c_clone = c.clone();
                                    move |_| editing.set(Some(c_clone.clone()))
                                },
                                on_delete: move |_| delete_id.set(Some(c.id)),
                            }
                        }
                    }
                }
            }

            // ST 式抽屉编辑器(右侧滑出浮层)
            if let Some(target) = editing() {
                div {
                    class: "absolute inset-0 z-30 flex justify-end bg-black/40 backdrop-blur-sm",
                    onclick: move |_| editing.set(None),
                    div {
                        class: "flex h-full w-full max-w-xl flex-col gap-4 border-l border-zinc-800 bg-zinc-900 p-5 shadow-2xl",
                        onclick: move |e| e.stop_propagation(),
                        CharacterEditor {
                            initial: target.clone(),
                            on_save: move |saved: Character| {
                                let mut list = characters.write();
                                if let Some(idx) = list.iter().position(|x| x.id == saved.id) {
                                    list[idx] = saved;
                                } else {
                                    list.push(saved);
                                }
                                editing.set(None);
                            },
                            on_cancel: move |_| editing.set(None),
                        }
                    }
                }
            }
        }

        // 删除确认弹窗
        Dialog {
            title: "删除角色".to_string(),
            open: delete_id().is_some(),
            on_cancel: move |_| delete_id.set(None),
            on_confirm: move |_| {
                if let Some(id) = delete_id() {
                    characters.write().retain(|c| c.id != id);
                    delete_id.set(None);
                }
            },
            "确定要删除该角色卡吗？本地对话记录将被保留。"
        }
    }
}

/// ST 风格大卡片:头像顶部 + 名字 + 标签 + 描述;hover 浮出操作。
#[component]
fn CharacterCard(
    char: Character,
    on_edit: EventHandler<MouseEvent>,
    on_delete: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "group relative flex flex-col overflow-hidden rounded-2xl border border-zinc-800/80 bg-zinc-900/60 transition-all hover:border-zinc-700 hover:bg-zinc-900",
            // 头部大头像/底色
            div { class: "relative flex h-28 w-full items-center justify-center bg-gradient-to-b from-zinc-800 to-zinc-900",
                Avatar {
                    name: char.name.clone(),
                    src: char.avatar_src.clone(),
                    size: "h-16 w-16 text-xl".to_string(),
                }
                // hover 操作浮层
                div { class: "absolute right-2 top-2 flex items-center gap-1 rounded-lg border border-zinc-800 bg-zinc-900/90 p-1 opacity-0 shadow-lg transition-opacity group-hover:opacity-100",
                    IconButton {
                        title: "编辑角色",
                        onclick: move |e| on_edit.call(e),
                        "✎"
                    }
                    IconButton {
                        title: "删除角色",
                        onclick: move |e| on_delete.call(e),
                        "✕"
                    }
                }
            }

            // 卡片内容
            div { class: "flex flex-1 flex-col gap-2 p-3.5",
                div { class: "flex items-baseline justify-between",
                    span { class: "truncate text-sm font-semibold text-zinc-100", "{char.name}" }
                }
                // 标签行
                if !char.tags.is_empty() {
                    div { class: "flex flex-wrap gap-1",
                        for t in char.tags.iter().take(3) {
                            span {
                                key: "{t}",
                                class: "rounded-md bg-zinc-800/90 px-1.5 py-0.5 text-[10px] text-zinc-400",
                                "#{t}"
                            }
                        }
                    }
                }
                // 描述
                p { class: "line-clamp-2 min-h-8 text-xs leading-4 text-zinc-400",
                    if char.description.is_empty() { "（无描述）" } else { "{char.description}" }
                }
            }
        }
    }
}

/// 六字段抽屉编辑器
#[component]
fn CharacterEditor(
    initial: Character,
    on_save: EventHandler<Character>,
    on_cancel: EventHandler<MouseEvent>,
) -> Element {
    let mut name = use_signal(|| initial.name.clone());
    let mut description = use_signal(|| initial.description.clone());
    let mut personality = use_signal(|| initial.personality.clone());
    let mut scenario = use_signal(|| initial.scenario.clone());
    let mut first_mes = use_signal(|| initial.first_mes.clone());
    let mut mes_example = use_signal(|| initial.mes_example.clone());
    let mut tags_str = use_signal(|| initial.tags.join(", "));

    let can_save = !name().trim().is_empty();

    rsx! {
        div { class: "flex h-full flex-col gap-4",
            div { class: "flex items-center justify-between border-b border-zinc-800 pb-3",
                span { class: "text-sm font-semibold text-zinc-100", "编辑角色卡" }
                div { class: "flex items-center gap-2",
                    button {
                        class: "rounded-full px-3 py-1 text-xs text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100",
                        onclick: move |e| on_cancel.call(e),
                        "取消"
                    }
                    button {
                        class: "rounded-full bg-zinc-100 px-3.5 py-1 text-xs font-semibold text-zinc-900 hover:bg-zinc-300 disabled:opacity-30",
                        disabled: !can_save,
                        onclick: move |_| {
                            let tags = tags_str()
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            on_save.call(Character {
                                id: initial.id,
                                name: name(),
                                avatar_src: initial.avatar_src.clone(),
                                description: description(),
                                personality: personality(),
                                scenario: scenario(),
                                first_mes: first_mes(),
                                mes_example: mes_example(),
                                tags,
                            });
                        },
                        "保存"
                    }
                }
            }

            div { class: "min-h-0 flex-1 space-y-3 overflow-y-auto pr-1 text-xs",
                Field { label: "角色名称 *",
                    input {
                        class: "w-full rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-zinc-100 outline-none focus:border-zinc-600",
                        placeholder: "如：巡音ルカ",
                        value: "{name()}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                Field { label: "标签 (逗号分隔)",
                    input {
                        class: "w-full rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-zinc-100 outline-none focus:border-zinc-600",
                        placeholder: "歌手, 冷静, VOCALOID",
                        value: "{tags_str()}",
                        oninput: move |e| tags_str.set(e.value()),
                    }
                }
                Field { label: "描述 (Description)",
                    textarea {
                        class: "h-20 w-full resize-none rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-zinc-100 outline-none focus:border-zinc-600",
                        placeholder: "外貌特征、背景故事、重要设定…",
                        value: "{description()}",
                        oninput: move |e| description.set(e.value()),
                    }
                }
                Field { label: "性格 (Personality)",
                    textarea {
                        class: "h-16 w-full resize-none rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-zinc-100 outline-none focus:border-zinc-600",
                        placeholder: "性格关键词、说话风格…",
                        value: "{personality()}",
                        oninput: move |e| personality.set(e.value()),
                    }
                }
                Field { label: "场景 (Scenario)",
                    textarea {
                        class: "h-16 w-full resize-none rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-zinc-100 outline-none focus:border-zinc-600",
                        placeholder: "当前对话发生的背景或契机…",
                        value: "{scenario()}",
                        oninput: move |e| scenario.set(e.value()),
                    }
                }
                Field { label: "开场白 (First Message)",
                    textarea {
                        class: "h-20 w-full resize-none rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-zinc-100 outline-none focus:border-zinc-600",
                        placeholder: "角色的第一句开场问候…",
                        value: "{first_mes()}",
                        oninput: move |e| first_mes.set(e.value()),
                    }
                }
                Field { label: "对话样例 (Mes Example)",
                    textarea {
                        class: "h-20 w-full resize-none rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 font-mono text-[11px] text-zinc-100 outline-none focus:border-zinc-600",
                        placeholder: "<START>\n{{user}}: 你好\n{{char}}: 很高兴见到你。",
                        value: "{mes_example()}",
                        oninput: move |e| mes_example.set(e.value()),
                    }
                }
            }
        }
    }
}
