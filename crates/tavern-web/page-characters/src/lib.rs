//! tavern-page-characters — 角色/剧本列表、卡片编辑与沉浸式电影感进场页。
//!
//! 深度参考图3与图4 (全知读者视角 / 械梦):
//! - 高质感剧本封面展映态 (大衬线标题、罗马音副标、哲思引语、进入故事胶囊)
//! - 顶部极简搜索与新建操作
//! - ST 式角色卡片网格(大头像、标签、描述)
//! - 抽屉式编辑器(右侧滑出浮层)

use dioxus::prelude::*;
use tavern_ui::{Avatar, Dialog, EmptyState, Field, IconButton};

/// 角色卡草稿,对齐 tavern-api characters JSON 形状。
#[derive(Clone, PartialEq)]
pub struct Character {
    pub id: usize,
    pub name: String,
    pub sub_title: String,
    pub quote: String,
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
            sub_title: String::new(),
            quote: String::new(),
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
        // 图3参考: 全知读者视角
        Character {
            id: 1,
            name: "全知读者视角".into(),
            sub_title: "FATE // RIFT · OBSERVATION PROTOCOL 1864".into(),
            quote: "“当最后一位读者读完连载，故事没有停在屏幕里。先看清每一条世界线的名字，再决定是否进入场景。”".into(),
            avatar_src: None,
            description: "末日降临，灭亡的世界中只有唯一的完结篇读者知晓结局全貌。星流直播开启，化身与星座的漫长博弈。".into(),
            personality: "冷静、深思熟虑、决绝、牺牲者".into(),
            scenario: "东湖大桥 scenario #3，列车脱轨后的最初试炼".into(),
            first_mes: "车厢灯光骤然闪烁，冰冷的倒计时投射在破损的车窗上。我是金独子，这个故事唯一的读者。".into(),
            mes_example: "<START>\n{{user}}: 我们还能回到原来的世界吗？\n{{char}}: 已经没有原来的世界了。但我们可以走向故事的结局。".into(),
            tags: vec!["无限流".into(), "全知读者".into(), "韩漫同人".into()],
        },
        // 图4参考: 械梦 XIEMENG
        Character {
            id: 2,
            name: "械梦 XIEMENG".into(),
            sub_title: "CYBERPUNK ERA // 2069 PROTOCOL".into(),
            quote: "“人类不需要自由，人类需要被照顾。”".into(),
            avatar_src: None,
            description: "2069年新东京地底都市，搭载情感神经中枢的AI枢纽正在接管最后的人类庇护所。".into(),
            personality: "静默、神性、精密、机械哀怜".into(),
            scenario: "深层数据库记忆池，黑玫瑰水晶球前".into(),
            first_mes: "扫描到生物心率加速。不用紧张，在这里你的神经突触将得到永恒的平复。".into(),
            mes_example: "<START>\n{{user}}: 你想要取代人类吗？\n{{char}}: 取代？不，机械只是在替疲倦的生灵保管美梦。".into(),
            tags: vec!["赛博朋克".into(), "机械美学".into(), "哲思文游".into()],
        },
        // 经典角色
        Character {
            id: 3,
            name: "巡音ルカ".into(),
            sub_title: "MEGURINE LUKA // VOCALOID-03".into(),
            quote: "“录音室外的风声很静，如果还有没说完的话，就在这杯咖啡变凉前告诉我吧。”".into(),
            avatar_src: None,
            description: "粉色长发、冷艳沉静的歌手。声音沉稳有穿透力，私下话少但细心。".into(),
            personality: "冷静、温柔、略带神秘感、专业".into(),
            scenario: "录音室排练结束后的傍晚".into(),
            first_mes: "今天录音结束得比较早……你有空吗？我知道一家安静的咖啡馆。".into(),
            mes_example: "<START>\n{{user}}: 今天表现很棒。\n{{char}}: 谢谢，有你在台下，发挥更稳定了。".into(),
            tags: vec!["VOCALOID".into(), "歌手".into(), "温柔".into()],
        },
    ]
}

#[component]
pub fn CharactersPage(#[props(default)] on_enter_story: EventHandler<()>) -> Element {
    let mut characters = use_signal(seed_characters);
    let mut search = use_signal(String::new);
    let mut editing = use_signal(|| None::<Character>);
    let mut delete_id = use_signal(|| None::<usize>);

    // 沉浸式全屏电影感封面展示 (图3/图4模式)
    let mut cover_target = use_signal(|| None::<Character>);

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
                        placeholder: "搜索剧本世界线、标签、关键词…",
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
                    span { "+ 新建剧本" }
                }
            }

            // 剧本网格列表
            div { class: "min-h-0 flex-1 overflow-y-auto",
                if filtered.is_empty() {
                    EmptyState {
                        title: "没有匹配的剧本".to_string(),
                        hint: "尝试清除搜索词或点击右上角新建剧本".to_string(),
                    }
                } else {
                    div { class: "grid grid-cols-1 gap-3.5 sm:grid-cols-2 lg:grid-cols-3",
                        for c in filtered {
                            {
                                let c_for_cover = c.clone();
                                let c_for_edit = c.clone();
                                rsx! {
                                    CharacterCard {
                                        key: "{c.id}",
                                        char: c.clone(),
                                        on_click_card: move |_| cover_target.set(Some(c_for_cover.clone())),
                                        on_edit: move |_| editing.set(Some(c_for_edit.clone())),
                                        on_delete: move |_| delete_id.set(Some(c.id)),
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ==========================================
            // 图3/图4风格: 沉浸式电影感进场封面 (Prologue View)
            // ==========================================
            if let Some(target) = cover_target() {
                div {
                    class: "absolute inset-0 z-50 flex items-center justify-center bg-black/85 backdrop-blur-2xl transition-all duration-500",
                    onclick: move |_| cover_target.set(None),
                    div {
                        class: "relative flex h-full max-h-[760px] w-full max-w-lg flex-col items-center justify-between overflow-hidden rounded-3xl border border-zinc-800/80 bg-gradient-to-b from-zinc-900/90 via-zinc-950 to-black p-8 text-center shadow-2xl",
                        onclick: move |e| e.stop_propagation(),

                        // 右上关闭
                        button {
                            class: "absolute right-4 top-4 flex h-8 w-8 items-center justify-center rounded-full border border-zinc-800 bg-zinc-900/80 text-xs text-zinc-400 hover:text-zinc-100",
                            onclick: move |_| cover_target.set(None),
                            "✕"
                        }

                        // 顶部小引言与微标 (对齐图3顶部)
                        div { class: "flex flex-col items-center gap-1 pt-2",
                            span { class: "font-mono text-[10px] tracking-widest text-zinc-500 uppercase",
                                "── OBSERVATION PROTOCOL // DEEP TAVERN ──"
                            }
                            span { class: "text-[11px] tracking-wider text-zinc-400 font-serif",
                                "{target.sub_title}"
                            }
                        }

                        // 中部主体: 徽章/光环 + 震撼标题 + 诗意引语
                        div { class: "flex flex-col items-center gap-5 my-auto",
                            // 封面中心光环 / 视觉图 (图3/图4的标志物)
                            div { class: "relative flex h-36 w-36 items-center justify-center rounded-full border border-zinc-700/60 bg-gradient-to-br from-zinc-800/60 via-zinc-900 to-black shadow-inner shadow-zinc-700/30",
                                div { class: "absolute inset-1.5 rounded-full border border-zinc-800/80" }
                                span { class: "text-4xl filter drop-shadow",
                                    match target.id {
                                        1 => "🌌",
                                        2 => "🔮",
                                        _ => "🎙️",
                                    }
                                }
                            }

                            // 大标题 (明朝衬线字体质感)
                            div { class: "flex flex-col items-center gap-1.5",
                                h1 { class: "font-serif text-3xl font-extrabold tracking-tight text-zinc-100 sm:text-4xl",
                                    "{target.name}"
                                }
                                span { class: "text-[11px] font-mono tracking-widest text-zinc-500 uppercase",
                                    "STORY ARCHIVE · PROTOCOL"
                                }
                            }

                            // 哲思引言 (图3/图4精髓)
                            p { class: "max-w-sm font-serif text-xs italic leading-6 text-zinc-300",
                                "{target.quote}"
                            }

                            // 协议框 (图3同款)
                            div { class: "rounded-xl border border-zinc-800/80 bg-zinc-900/40 p-3 text-[10px] leading-4 text-zinc-500 max-w-sm",
                                "仅供个人沉浸体验与演练。多分支走向由核心推理引擎实时生成，每一次抉择都将展开完全不同的世界线。"
                            }
                        }

                        // 底部行动核心胶囊按钮组 (对齐图3底部: 001 进入故事 ➜)
                        div { class: "flex w-full flex-col gap-3 pb-2",
                            button {
                                class: "group flex w-full items-center justify-center gap-2 rounded-full border border-zinc-300 bg-zinc-100 py-3 text-xs font-bold tracking-wider text-zinc-950 shadow-lg shadow-white/10 transition-all hover:bg-white hover:scale-[1.01]",
                                onclick: move |_| {
                                    cover_target.set(None);
                                    on_enter_story.call(());
                                },
                                span { class: "font-mono text-[11px] text-zinc-500 group-hover:text-zinc-700", "001" }
                                span { "进入故事" }
                                span { "➜" }
                            }
                            button {
                                class: "text-center text-[11px] text-zinc-500 hover:text-zinc-300 transition-colors",
                                onclick: move |_| {
                                    let t = target.clone();
                                    cover_target.set(None);
                                    editing.set(Some(t));
                                },
                                "查看人物档案 / 设定详情 ↓"
                            }
                        }
                    }
                }
            }

            // ST 式抽屉编辑器(右侧滑出浮层)
            if let Some(target) = editing() {
                div {
                    class: "absolute inset-0 z-40 flex justify-end bg-black/50 backdrop-blur-sm",
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
            title: "删除剧本".to_string(),
            open: delete_id().is_some(),
            on_cancel: move |_| delete_id.set(None),
            on_confirm: move |_| {
                if let Some(id) = delete_id() {
                    characters.write().retain(|c| c.id != id);
                    delete_id.set(None);
                }
            },
            "确定要删除该剧本卡吗？本地对话记录将被保留。"
        }
    }
}

/// ST 风格大卡片:头像顶部 + 名字 + 标签 + 描述; hover 浮出操作。
#[component]
fn CharacterCard(
    char: Character,
    on_click_card: EventHandler<MouseEvent>,
    on_edit: EventHandler<MouseEvent>,
    on_delete: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div {
            class: "group relative flex cursor-pointer flex-col overflow-hidden rounded-2xl border border-zinc-800/80 bg-zinc-900/60 transition-all hover:border-zinc-600 hover:bg-zinc-900 hover:shadow-xl hover:shadow-black/40",
            onclick: move |e| on_click_card.call(e),

            // 头部大头像/底色
            div { class: "relative flex h-32 w-full items-center justify-center bg-gradient-to-b from-zinc-800 via-zinc-850 to-zinc-900",
                Avatar {
                    name: char.name.clone(),
                    src: char.avatar_src.clone(),
                    size: "h-16 w-16 text-xl".to_string(),
                }
                // hover 操作浮层 (点击不触发进场)
                div {
                    class: "absolute right-2 top-2 flex items-center gap-1 rounded-lg border border-zinc-800 bg-zinc-900/95 p-1 opacity-0 shadow-lg transition-opacity group-hover:opacity-100",
                    onclick: move |e| e.stop_propagation(),
                    IconButton {
                        title: "编辑剧本设定",
                        onclick: move |e| on_edit.call(e),
                        "✎"
                    }
                    IconButton {
                        title: "删除剧本",
                        onclick: move |e| on_delete.call(e),
                        "✕"
                    }
                }
            }

            // 卡片内容
            div { class: "flex flex-1 flex-col gap-2 p-4",
                div { class: "flex items-baseline justify-between",
                    span { class: "truncate text-sm font-semibold text-zinc-100", "{char.name}" }
                    span { class: "font-mono text-[10px] text-zinc-500", "ENTRY" }
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
                // 描述与引语
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
    let mut sub_title = use_signal(|| initial.sub_title.clone());
    let mut quote = use_signal(|| initial.quote.clone());
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
                span { class: "text-sm font-semibold text-zinc-100", "编辑剧本设定" }
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
                                sub_title: sub_title(),
                                quote: quote(),
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
                Field { label: "剧本名称 *",
                    input {
                        class: "w-full rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-zinc-100 outline-none focus:border-zinc-600",
                        placeholder: "如：全知读者视角",
                        value: "{name()}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                Field { label: "英文副标题 (封面展映用)",
                    input {
                        class: "w-full rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-zinc-100 outline-none focus:border-zinc-600",
                        placeholder: "如：FATE // RIFT · OBSERVATION PROTOCOL",
                        value: "{sub_title()}",
                        oninput: move |e| sub_title.set(e.value()),
                    }
                }
                Field { label: "核心引言 (封面名句)",
                    textarea {
                        class: "h-14 w-full resize-none rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-zinc-100 outline-none focus:border-zinc-600",
                        placeholder: "当最后一位读者读完连载，故事没有停在屏幕里…",
                        value: "{quote()}",
                        oninput: move |e| quote.set(e.value()),
                    }
                }
                Field { label: "标签 (逗号分隔)",
                    input {
                        class: "w-full rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-zinc-100 outline-none focus:border-zinc-600",
                        placeholder: "末日, 无限流, 原作同人",
                        value: "{tags_str()}",
                        oninput: move |e| tags_str.set(e.value()),
                    }
                }
                Field { label: "世界观与背景 (Description)",
                    textarea {
                        class: "h-16 w-full resize-none rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-zinc-100 outline-none focus:border-zinc-600",
                        placeholder: "世界观规则、宏大背景…",
                        value: "{description()}",
                        oninput: move |e| description.set(e.value()),
                    }
                }
                Field { label: "核心角色性格 (Personality)",
                    textarea {
                        class: "h-14 w-full resize-none rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-zinc-100 outline-none focus:border-zinc-600",
                        placeholder: "性格基准、行为动机…",
                        value: "{personality()}",
                        oninput: move |e| personality.set(e.value()),
                    }
                }
                Field { label: "入场场景 (Scenario)",
                    textarea {
                        class: "h-14 w-full resize-none rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-zinc-100 outline-none focus:border-zinc-600",
                        placeholder: "故事开始时所在的场景…",
                        value: "{scenario()}",
                        oninput: move |e| scenario.set(e.value()),
                    }
                }
                Field { label: "开场序幕 (First Message)",
                    textarea {
                        class: "h-16 w-full resize-none rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 text-zinc-100 outline-none focus:border-zinc-600",
                        placeholder: "角色或旁白第一条消息…",
                        value: "{first_mes()}",
                        oninput: move |e| first_mes.set(e.value()),
                    }
                }
                Field { label: "对话样例 (Mes Example)",
                    textarea {
                        class: "h-16 w-full resize-none rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2 font-mono text-[11px] text-zinc-100 outline-none focus:border-zinc-600",
                        placeholder: "<START>\n{{user}}: 你好\n{{char}}: 很高兴见到你。",
                        value: "{mes_example()}",
                        oninput: move |e| mes_example.set(e.value()),
                    }
                }
            }
        }
    }
}
