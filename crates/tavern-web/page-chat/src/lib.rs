//! tavern-page-chat — 文游与角色扮演互动界面。
//!
//! 深度响应用户需求1:
//! 1. 顶部纯净精简，轮次/分页移至顶层左侧 (`‹ 第 1 轮 · 跳至 1 页 ›`)
//! 2. 模型切换胶囊下移到**输入框正上方**工具栏，并搭配高频互动快捷按钮 (调查现场、追问细节、推进剧情)
//! 3. 剧情主舞台: 任务系统卡片、线索备忘录卡片、沉浸分支决策卡片
//! 4. 右上角快捷菜单 (导出、导入、记忆设定、模型参数等)

use dioxus::prelude::*;
use tavern_ui::{ChoiceCard, ChoiceOption, Dialog, IconButton, MessageBubble, StatusCard, SwipePicker};

/// 会话项
#[derive(Clone, PartialEq)]
struct SessionItem {
    id: usize,
    title: String,
    updated_at: String,
}

/// 消息流项目类型
#[derive(Clone, PartialEq)]
enum StoryItem {
    Dialogue {
        id: usize,
        name: String,
        time: String,
        content: String,
        mine: bool,
        swipes: Vec<String>,
        swipe_idx: usize,
    },
    SystemCard {
        id: usize,
        title: String,
        color: String,
        content: String,
    },
    PlayerChoice {
        id: usize,
        title: String,
        options: Vec<ChoiceOption>,
    },
}

fn seed_sessions() -> Vec<SessionItem> {
    vec![
        SessionItem {
            id: 1,
            title: "新的对话-1 (当前时间线)".into(),
            updated_at: "刚刚".into(),
        },
        SessionItem {
            id: 2,
            title: "平行线: 决战前夕对赌协议".into(),
            updated_at: "昨天".into(),
        },
        SessionItem {
            id: 3,
            title: "分支: 拒绝资本联合出品".into(),
            updated_at: "3天前".into(),
        },
    ]
}

fn seed_story_items() -> Vec<StoryItem> {
    vec![
        StoryItem::SystemCard {
            id: 1,
            title: "📜 通告与对赌 (任务系统)".into(),
            color: "emerald".into(),
            content: "【主线契约】当前处于第 3 轮博弈阶段：需在今晚 24:00 前迫使资方代表签署独立制片人豁免协议，对赌筹码为 20% 分成收益与下半年 S 级项目主控权。".into(),
        },
        StoryItem::Dialogue {
            id: 2,
            name: "顾清言 (资方代表)".into(),
            time: "21:40".into(),
            content: "“你以为带着一份没有背书的企划案，就能说服董事会放权？”\n她将温热的骨瓷茶杯轻轻搁在黑胡桃木桌上，清脆的微响在沉寂的套房里格外清晰。她的目光越过镜片，带着审视与不易察觉的戒备。".into(),
            mine: false,
            swipes: vec![
                "“你以为带着一份没有背书的企划案，就能说服董事会放权？”\n她将温热的骨瓷茶杯轻轻搁在黑胡桃木桌上，清脆的微响在沉寂的套房里格外清晰。她的目光越过镜片，带着审视与不易察觉的戒备。".into(),
                "“正午阳光那边的反馈已经递到我桌上了，他们对你的风格有疑虑。”\n她语气平淡地抽出一份蓝色文件夹，指尖在签名页停顿了片刻。".into(),
            ],
            swipe_idx: 0,
        },
        StoryItem::SystemCard {
            id: 3,
            title: "💾 娱乐圈备忘录 (记忆区)".into(),
            color: "amber".into(),
            content: "【线索捕捉】对方指尖微颤，视线两次避开左手腕的百达翡丽。她今晚承受着来自集团总部的连带清算压力，并非不可攻破。".into(),
        },
        StoryItem::PlayerChoice {
            id: 4,
            title: "玩家决策 (行动选项)".into(),
            options: vec![
                ChoiceOption {
                    key: "A".into(),
                    text: "极度冷酷地挑明正午阳光对她抗拒的真相，逼迫她拿出更具实质性的资本筹码。".into(),
                },
                ChoiceOption {
                    key: "B".into(),
                    text: "缓和气氛，以资方掌权者的从容姿态倒茶，暗示可以通过“私人层面的深度合作”打破僵局。".into(),
                },
                ChoiceOption {
                    key: "C".into(),
                    text: "剥离伪装，直接用目光和言语施压，要求她卸下高定外套的防备，开门见山谈底价。".into(),
                },
                ChoiceOption {
                    key: "D".into(),
                    text: "暂时搁置谈判，借故起身接听腾讯视频高层的内部电话，以此施加时间压力。".into(),
                },
                ChoiceOption {
                    key: "E".into(),
                    text: "ALL IN (自由输入你的行动，亲自打破规则)".into(),
                },
            ],
        },
    ]
}

#[component]
pub fn ChatPage(
    #[props(default)] on_goto_characters: EventHandler<()>,
    #[props(default)] on_goto_settings: EventHandler<()>,
    #[props(default)] on_goto_home: EventHandler<()>,
    #[props(default)] on_toggle_theme: EventHandler<()>,
    #[props(default = false)] theme_light: bool,
) -> Element {
    let mut sessions = use_signal(seed_sessions);
    let mut current_session_id = use_signal(|| 1usize);
    let mut story_items = use_signal(seed_story_items);

    // 界面控制状态
    let mut sidebar_open = use_signal(|| true);
    let mut menu_open = use_signal(|| false);
    let mut current_model = use_signal(|| "gemini-3-flash-preview".to_string());
    let mut model_dropdown_open = use_signal(|| false);

    // 工具栏开关
    let mut memory_boost = use_signal(|| true);
    let mut stream_toggle = use_signal(|| true);
    let mut mod_active = use_signal(|| false);

    // 输入与弹窗
    let mut draft = use_signal(String::new);
    let mut delete_id = use_signal(|| None::<usize>);

    let models = vec![
        "gemini-3-flash-preview",
        "claude-3-5-sonnet",
        "deepseek-chat",
        "gpt-4o",
    ];

    let mut handle_send = move || {
        let text = draft().trim().to_string();
        if text.is_empty() {
            return;
        }
        let next_id = story_items().len() + 1;
        story_items.write().push(StoryItem::Dialogue {
            id: next_id,
            name: "我 (玩家)".into(),
            time: "刚刚".into(),
            content: text,
            mine: true,
            swipes: vec![],
            swipe_idx: 0,
        });
        draft.set(String::new());
    };

    rsx! {
        div { class: "relative flex h-full w-full overflow-hidden bg-zinc-950 text-zinc-100",
            // ==========================================
            // 左侧可折叠剧本与会话侧栏
            // ==========================================
            div {
                class: if sidebar_open() {
                    "relative flex h-full w-72 shrink-0 flex-col border-r border-zinc-800/80 bg-zinc-900/70 backdrop-blur-xl transition-all duration-300 z-20"
                } else {
                    "relative flex h-full w-0 shrink-0 flex-col overflow-hidden border-r-0 border-zinc-800/0 bg-transparent transition-all duration-300 z-20"
                },
                if sidebar_open() {
                    div { class: "flex h-full w-72 flex-col gap-4 p-4",
                        // 剧本标题与折叠
                        div { class: "flex items-start justify-between gap-2 border-b border-zinc-800/80 pb-3",
                            div { class: "flex flex-col gap-1",
                                span { class: "line-clamp-2 text-xs font-bold leading-5 tracking-tight text-zinc-100",
                                    "【超真实】明星娱乐圈模拟器"
                                }
                                span { class: "text-[10px] text-zinc-500", "当代全球演艺资本衍生规则" }
                            }
                            button {
                                class: "flex h-6 w-6 shrink-0 items-center justify-center rounded-md text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100",
                                title: "折叠侧栏",
                                onclick: move |_| sidebar_open.set(false),
                                "«"
                            }
                        }

                        div { class: "rounded-xl border border-zinc-800/60 bg-zinc-950/40 p-2.5 text-[11px] leading-4 text-zinc-400",
                            "【细腻UI和美化】【真实数据库与衍生规则】一比一复刻当代娱乐产业生态。这里有冰冷的资本运作与残酷的名利场。"
                        }

                        div { class: "grid grid-cols-2 gap-2",
                            button {
                                class: "flex items-center justify-center gap-1 rounded-xl border border-zinc-800 bg-zinc-800/60 py-1.5 text-xs text-zinc-300 transition-colors hover:bg-zinc-700",
                                onclick: move |_| on_goto_characters.call(()),
                                "📖 剧本详情"
                            }
                            button { class: "flex items-center justify-center gap-1 rounded-xl border border-amber-500/30 bg-amber-500/10 py-1.5 text-xs text-amber-300 transition-colors hover:bg-amber-500/20",
                                "☕ 赞赏作品"
                            }
                        }

                        // 会话列表
                        div { class: "flex min-h-0 flex-1 flex-col gap-2 pt-2",
                            div { class: "flex items-center justify-between px-1",
                                span { class: "text-xs font-semibold text-zinc-400", "会话时间线" }
                                button {
                                    class: "flex items-center gap-1 rounded-lg bg-zinc-800 px-2 py-0.5 text-[11px] font-medium text-zinc-200 hover:bg-zinc-700",
                                    onclick: move |_| {
                                        let next_id = sessions().len() + 1;
                                        sessions.write().insert(0, SessionItem {
                                            id: next_id,
                                            title: format!("分支行动-{}", next_id),
                                            updated_at: "刚刚".into(),
                                        });
                                        current_session_id.set(next_id);
                                    },
                                    "+ 新分支"
                                }
                            }
                            div { class: "flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto pr-1",
                                for s in sessions() {
                                    {
                                        let is_active = current_session_id() == s.id;
                                        let s_id = s.id;
                                        rsx! {
                                            button {
                                                key: "{s.id}",
                                                class: if is_active {
                                                    "group flex w-full flex-col gap-1 rounded-xl border border-zinc-700 bg-zinc-800/90 p-2.5 text-left shadow-sm"
                                                } else {
                                                    "group flex w-full flex-col gap-1 rounded-xl border border-transparent p-2.5 text-left text-zinc-400 hover:border-zinc-800 hover:bg-zinc-900/60"
                                                },
                                                onclick: move |_| current_session_id.set(s_id),
                                                div { class: "flex items-center justify-between",
                                                    span { class: "truncate text-xs font-medium text-zinc-200", "{s.title}" }
                                                    span { class: "text-[10px] text-zinc-500", "{s.updated_at}" }
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

            // ==========================================
            // 中央互动剧情主视区
            // ==========================================
            div { class: "relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden",
                // 顶部一体化微条 (需求1: 轮次/分页移至顶层，与剧本库及设置协同)
                div { class: "flex h-12 shrink-0 items-center justify-between border-b border-zinc-800/60 bg-zinc-900/60 px-4 backdrop-blur-xl z-10",
                    // 左侧：回到主页/剧本库 + 展开侧栏 + 轮次分页 (对齐需求1)
                    div { class: "flex items-center gap-2",
                        // 点击 FERRITE 直接回到品牌主页 (对齐需求4)
                        button {
                            class: "flex items-center gap-1.5 rounded-lg border border-purple-500/30 bg-purple-950/40 px-2.5 py-1 text-xs text-purple-200 hover:bg-purple-900/40 transition-colors font-serif font-bold tracking-wider",
                            title: "回到 Tavern 官方品牌主页",
                            onclick: move |_| on_goto_home.call(()),
                            span { "🏛️" }
                            span { "主页" }
                        }
                        button {
                            class: "flex h-7 items-center gap-1.5 rounded-lg border border-zinc-800 bg-zinc-900/80 px-2.5 text-xs text-zinc-300 hover:bg-zinc-800 transition-colors",
                            title: "回到剧本库网格",
                            onclick: move |_| on_goto_characters.call(()),
                            span { "📚" }
                            span { class: "font-medium hidden sm:inline", "剧本库" }
                        }
                        if !sidebar_open() {
                            button {
                                class: "flex h-7 items-center gap-1 rounded-lg border border-zinc-800 bg-zinc-900/80 px-2 text-xs text-zinc-400 hover:bg-zinc-800",
                                title: "展开侧栏",
                                onclick: move |_| sidebar_open.set(true),
                                span { "»" }
                            }
                        }

                        // 顶层左侧轮次与分页器 (对齐需求1)
                        div { class: "flex items-center gap-1.5 rounded-full border border-zinc-800 bg-zinc-950/70 px-2.5 py-0.5 text-zinc-400 text-xs ml-1",
                            button { class: "hover:text-zinc-200 px-0.5", "‹" }
                            span { class: "text-[10px] font-medium tabular-nums text-zinc-200", "第 1 轮 · 跳至 1 页" }
                            button { class: "hover:text-zinc-200 px-0.5", "›" }
                        }
                    }

                    // 右侧：主题微按钮 + 快捷菜单齿轮
                    div { class: "flex items-center gap-2",
                        button {
                            class: "flex h-7 w-7 items-center justify-center rounded-lg text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-200 text-xs",
                            title: "切换光暗模式",
                            onclick: move |_| on_toggle_theme.call(()),
                            if theme_light { "☀️" } else { "🌙" }
                        }
                        button {
                            class: "flex h-7 w-7 items-center justify-center rounded-lg border border-zinc-800 bg-zinc-900 text-xs text-zinc-300 transition-colors hover:bg-zinc-800 hover:text-zinc-100",
                            title: "快捷设置菜单",
                            onclick: move |_| menu_open.set(!menu_open()),
                            "⚙"
                        }
                    }
                }

                // 剧情与卡片主滚动流
                div { class: "flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-4 sm:p-6 lg:px-20 xl:px-32",
                    for item in story_items.read().clone() {
                        match item {
                            StoryItem::SystemCard { id: _, title, color, content } => rsx! {
                                StatusCard { title, color, "{content}" }
                            },
                            StoryItem::Dialogue { id, name, time, content, mine, swipes, swipe_idx } => rsx! {
                                MessageBubble {
                                    key: "{id}",
                                    name: name.clone(),
                                    time: time.clone(),
                                    content: content.clone(),
                                    mine,
                                    actions: rsx! {
                                        if !mine && swipes.len() > 1 {
                                            SwipePicker {
                                                index: swipe_idx,
                                                total: swipes.len(),
                                                on_prev: move |_| {},
                                                on_next: move |_| {},
                                            }
                                        }
                                        IconButton {
                                            title: "删除该条",
                                            onclick: move |_| delete_id.set(Some(id)),
                                            "✕"
                                        }
                                    },
                                }
                            },
                            StoryItem::PlayerChoice { id: _, title, options } => rsx! {
                                ChoiceCard {
                                    title,
                                    options,
                                    on_select: move |chosen_text: String| {
                                        draft.set(chosen_text);
                                        handle_send();
                                    },
                                }
                            },
                        }
                    }
                }

                // ==========================================
                // 底部输入区与控制工具岛 (对齐需求1: 模型切换放输入框上方，增加快捷操作按钮行)
                // ==========================================
                div { class: "flex shrink-0 flex-col gap-2 border-t border-zinc-800/60 bg-zinc-900/80 p-3 backdrop-blur-2xl",
                    // 1. 输入框正上方: 模型切换胶囊 + 文游高频快捷交互动作条 (对齐需求1)
                    div { class: "flex flex-wrap items-center justify-between gap-2 px-1 text-xs",
                        // 左侧: 居中的模型切换胶囊下移至此 (对齐需求1)
                        div { class: "relative",
                            button {
                                class: "flex items-center gap-1.5 rounded-full border border-purple-500/40 bg-zinc-950/80 px-3 py-1 text-xs font-semibold text-purple-200 shadow-sm transition-all hover:border-purple-400",
                                onclick: move |_| model_dropdown_open.set(!model_dropdown_open()),
                                span { "⚡" }
                                span { "{current_model()}" }
                                span { class: "text-[10px] text-zinc-400", "⌵" }
                            }
                            if model_dropdown_open() {
                                div { class: "absolute bottom-full left-0 z-50 mb-2 w-56 flex-col rounded-xl border border-zinc-800 bg-zinc-900 p-1 shadow-2xl backdrop-blur-2xl",
                                    for m in models.clone() {
                                        {
                                            let model_name = m.to_string();
                                            rsx! {
                                                button {
                                                    key: "{m}",
                                                    class: "flex w-full items-center justify-between rounded-lg px-2.5 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800 hover:text-zinc-100",
                                                    onclick: move |_| {
                                                        current_model.set(model_name.clone());
                                                        model_dropdown_open.set(false);
                                                    },
                                                    span { "{m}" }
                                                    if current_model() == m {
                                                        span { class: "text-[10px] text-emerald-400", "✓" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // 中部快捷动作胶囊组 (文游高频动作 + 开关)
                        div { class: "flex flex-wrap items-center gap-1.5",
                            button {
                                class: "rounded-full border border-zinc-800 bg-zinc-950/70 px-2.5 py-1 text-[11px] text-zinc-300 hover:bg-zinc-800 hover:text-white transition-colors",
                                onclick: move |_| {
                                    draft.set("【敏锐洞察】仔细打量四周环境与对方微妙的肢体反应。".into());
                                    handle_send();
                                },
                                "🔍 观察现场"
                            }
                            button {
                                class: "rounded-full border border-zinc-800 bg-zinc-950/70 px-2.5 py-1 text-[11px] text-zinc-300 hover:bg-zinc-800 hover:text-white transition-colors",
                                onclick: move |_| {
                                    draft.set("【深入追问】“你刚才的话，似乎并没有说完。”".into());
                                    handle_send();
                                },
                                "🗣️ 深入追问"
                            }
                            button {
                                class: "rounded-full border border-zinc-800 bg-zinc-950/70 px-2.5 py-1 text-[11px] text-zinc-300 hover:bg-zinc-800 hover:text-white transition-colors",
                                onclick: move |_| {
                                    draft.set("【推进剧情】沉默数秒后，直接切入核心条款。".into());
                                    handle_send();
                                },
                                "⏩ 推进剧情"
                            }

                            // 状态微开关
                            button {
                                class: if mod_active() {
                                    "rounded-full border border-purple-500/40 bg-purple-500/20 px-2.5 py-1 text-[11px] font-medium text-purple-200"
                                } else {
                                    "rounded-full border border-zinc-800 bg-zinc-950/70 px-2.5 py-1 text-[11px] text-zinc-400 hover:text-zinc-200"
                                },
                                onclick: move |_| mod_active.set(!mod_active()),
                                "🎮 Mod"
                            }
                            button {
                                class: if memory_boost() {
                                    "rounded-full border border-emerald-500/40 bg-emerald-500/20 px-2.5 py-1 text-[11px] font-medium text-emerald-200"
                                } else {
                                    "rounded-full border border-zinc-800 bg-zinc-950/70 px-2.5 py-1 text-[11px] text-zinc-400 hover:text-zinc-200"
                                },
                                onclick: move |_| memory_boost.set(!memory_boost()),
                                "🧠 记忆"
                            }
                            button {
                                class: if stream_toggle() {
                                    "rounded-full border border-cyan-500/40 bg-cyan-500/20 px-2.5 py-1 text-[11px] font-medium text-cyan-200"
                                } else {
                                    "rounded-full border border-zinc-800 bg-zinc-950/70 px-2.5 py-1 text-[11px] text-zinc-400 hover:text-zinc-200"
                                },
                                onclick: move |_| stream_toggle.set(!stream_toggle()),
                                "≈ 流式"
                            }
                            button {
                                class: "rounded-full border border-rose-500/30 bg-rose-500/10 px-2.5 py-1 text-[11px] font-medium text-rose-300 hover:bg-rose-500/20",
                                onclick: move |_| {
                                    draft.set("【突发离场】拒绝此项提议，直接推门离场。".into());
                                    handle_send();
                                },
                                "跑路！！！"
                            }
                        }
                    }

                    // 2. 输入框与发送按钮
                    div { class: "flex items-end gap-2 rounded-2xl border border-zinc-800 bg-zinc-950/90 p-2.5 shadow-inner",
                        textarea {
                            class: "h-11 min-h-11 flex-1 resize-none rounded-xl bg-transparent px-3 py-2 text-sm text-zinc-100 outline-none placeholder:text-zinc-600 focus:ring-0",
                            placeholder: "点击上方行动选项，或输入你的自定义决策 (电脑端 Shift+回车可换行)",
                            value: "{draft()}",
                            oninput: move |e| draft.set(e.value()),
                            onkeydown: move |e| {
                                if e.key() == Key::Enter && !e.modifiers().shift() {
                                    e.prevent_default();
                                    handle_send();
                                }
                            },
                        }
                        div { class: "flex shrink-0 items-center gap-2",
                            span { class: "text-[10px] text-zinc-600 tabular-nums", "{draft().len()}" }
                            button {
                                class: "flex h-9 items-center justify-center rounded-full bg-gradient-to-r from-purple-600 to-pink-600 px-5 text-xs font-bold text-white shadow-md shadow-purple-600/30 transition-all hover:scale-105 hover:shadow-purple-600/50 disabled:opacity-40",
                                disabled: draft().trim().is_empty(),
                                onclick: move |_| handle_send(),
                                "行动 ➜"
                            }
                        }
                    }
                }

                // 右上角抽屉快捷菜单
                if menu_open() {
                    div {
                        class: "absolute inset-0 z-40 bg-black/20",
                        onclick: move |_| menu_open.set(false),
                    }
                    div { class: "absolute right-3 top-14 z-50 flex w-48 flex-col divide-y divide-zinc-800 rounded-2xl border border-zinc-800 bg-zinc-900/95 p-1.5 shadow-2xl backdrop-blur-2xl text-xs",
                        div { class: "flex flex-col py-1",
                            button { class: "flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-zinc-300 hover:bg-zinc-800",
                                "📤 导出记录"
                            }
                            button { class: "flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-zinc-300 hover:bg-zinc-800",
                                "📥 导入聊天记录"
                            }
                            button { class: "flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-zinc-300 hover:bg-zinc-800",
                                "🔗 分享作品"
                            }
                        }
                        div { class: "flex flex-col py-1",
                            button { class: "flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-zinc-300 hover:bg-zinc-800",
                                "🧠 记忆设定"
                            }
                            button { class: "flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-zinc-300 hover:bg-zinc-800",
                                "🎨 自定义配图"
                            }
                        }
                        div { class: "flex flex-col py-1",
                            button {
                                class: "flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-zinc-200 font-medium hover:bg-zinc-800",
                                onclick: move |_| {
                                    menu_open.set(false);
                                    on_goto_settings.call(());
                                },
                                "⚙️ 模型与采样参数"
                            }
                        }
                    }
                }
            }
        }

        // 删除确认弹窗
        Dialog {
            title: "删除消息段落".to_string(),
            open: delete_id().is_some(),
            on_cancel: move |_| delete_id.set(None),
            on_confirm: move |_| {
                if let Some(id) = delete_id() {
                    story_items.write().retain(|item| {
                        match item {
                            StoryItem::Dialogue { id: mid, .. } => *mid != id,
                            StoryItem::SystemCard { id: sid, .. } => *sid != id,
                            StoryItem::PlayerChoice { id: cid, .. } => *cid != id,
                        }
                    });
                    delete_id.set(None);
                }
            },
            "确定要从当前时间线中删除此段剧情吗？"
        }
    }
}
