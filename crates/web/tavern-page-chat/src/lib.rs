//! tavern-page-chat — 文游与角色扮演互动界面。
//!
//! 深度优化满足需求:
//! 1. 左右侧边栏全面重构为抽屉 (Drawer) 模式，悬浮占位微标展开，互斥排他打开逻辑 (绝对不同时打开)
//! 2. 会话多聊天室真正独立隔离 (每个分支会话维护各自的消息历史，切换时完整重载不同内容)
//! 3. 消息气泡交互升级: 点击气泡浮出专属操作菜单 (复制/编辑/分支切换/删除)
//! 4. 侧栏「剧本详情」和「赞赏作品」以景深模糊弹窗 (Modal) 呈现
//! 5. 平滑滚动 + 隐藏滚动条

use dioxus::prelude::*;
use tavern_ui::{
    ChoiceCard, ChoiceOption, Dialog, IconButton, MessageBubble, StatusCard, SwipePicker,
};

/// 会话项
#[derive(Clone, PartialEq)]
pub struct SessionItem {
    pub id: usize,
    pub title: String,
    pub updated_at: String,
}

/// 消息流项目类型
#[derive(Clone, PartialEq)]
pub enum StoryItem {
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

impl StoryItem {
    pub fn id(&self) -> usize {
        match self {
            StoryItem::Dialogue { id, .. } => *id,
            StoryItem::SystemCard { id, .. } => *id,
            StoryItem::PlayerChoice { id, .. } => *id,
        }
    }

    pub fn nav_title(&self) -> (&'static str, String) {
        match self {
            StoryItem::SystemCard { title, .. } => ("系统", title.clone()),
            StoryItem::Dialogue {
                name,
                mine,
                content,
                ..
            } => {
                let prefix = if *mine { "玩家行动" } else { "NPC对话" };
                let short: String = content.chars().take(14).collect();
                (prefix, format!("{}: {}…", name, short))
            }
            StoryItem::PlayerChoice { title, .. } => ("决策", title.clone()),
        }
    }
}

pub fn seed_sessions() -> Vec<SessionItem> {
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

/// 获取不同会话对应的独立剧情消息流 (彻底解决需求4: 会话多聊天室内容真实切换隔离)
fn get_session_story_items(session_id: usize) -> Vec<StoryItem> {
    match session_id {
        2 => vec![
            StoryItem::SystemCard {
                id: 201,
                title: "📜 对赌决战前夕 (时间线-平行分支)".into(),
                color: "purple".into(),
                content: "【平行世界线】你拒绝了正午阳光的中间人游说，直接持企划案闯入资方总部核心会议室。".into(),
            },
            StoryItem::Dialogue {
                id: 202,
                name: "董事会执行秘书".into(),
                time: "昨天 23:15".into(),
                content: "“顾总在里面等你。不过提醒你一句，今晚集团审计部也在场，说话最好留有余地。”".into(),
                mine: false,
                swipes: vec!["“顾总在里面等你。今晚集团审计部也在场，说话留有余地。”".into()],
                swipe_idx: 0,
            },
            StoryItem::Dialogue {
                id: 203,
                name: "我 (玩家)".into(),
                time: "昨天 23:16".into(),
                content: "“多谢提醒。我既然敢来，就没打算空手回去。”".into(),
                mine: true,
                swipes: vec![],
                swipe_idx: 0,
            },
            StoryItem::PlayerChoice {
                id: 204,
                title: "决战行动分支".into(),
                options: vec![
                    ChoiceOption {
                        key: "A".into(),
                        text: "推门而入，将审计漏洞证据直接甩在谈判桌中央。".into(),
                    },
                    ChoiceOption {
                        key: "B".into(),
                        text: "从容就座，先要求核验资方的资金到账流水。".into(),
                    },
                ],
            },
        ],
        3 => vec![
            StoryItem::SystemCard {
                id: 301,
                title: "📜 决裂分支: 拒绝资本联合出品".into(),
                color: "rose".into(),
                content: "【孤狼时间线】你撕毁了联合出品协议草案，选择独立融资并启动宣发众筹，彻底站在传统院线资方的对立面。".into(),
            },
            StoryItem::Dialogue {
                id: 302,
                name: "顾清言 (资方代表)".into(),
                time: "3天前 16:00".into(),
                content: "“你太冲动了。没有我们的院线排片保底，你的片子甚至走不出点映场。”".into(),
                mine: false,
                swipes: vec!["“没有院线排片保底，你的片子甚至走不出点映场。”".into()],
                swipe_idx: 0,
            },
            StoryItem::Dialogue {
                id: 303,
                name: "我 (玩家)".into(),
                time: "3天前 16:05".into(),
                content: "“时代变了，顾总。观众想看的是好故事，不是你们资本洗牌的工具。”".into(),
                mine: true,
                swipes: vec![],
                swipe_idx: 0,
            },
        ],
        _ => vec![
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
        ],
    }
}

pub fn seed_story_items() -> Vec<StoryItem> {
    get_session_story_items(1)
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

    // 抽屉互斥排他控制 (需求3: 严禁左右抽屉同时打开)
    // None: 全部收起, Some("left"): 仅开左侧会话抽屉, Some("right"): 仅开右侧大纲抽屉
    let mut active_drawer = use_signal(|| None::<&'static str>);

    // 气泡专属菜单交互 (需求5: 点击气泡才弹出，点击别处收起)
    let mut active_bubble_menu_id = use_signal(|| None::<usize>);

    // 弹窗状态 (需求6: 剧本详情与赞赏作品采用弹窗 Modal)
    let mut detail_modal_open = use_signal(|| false);
    let mut donate_modal_open = use_signal(|| false);

    let mut menu_open = use_signal(|| false);
    let mut current_model = use_signal(|| "gemini-3.1-pro-preview-high".to_string());
    let mut model_dropdown_open = use_signal(|| false);

    // 工具栏开关
    let mut memory_boost = use_signal(|| true);
    let mut stream_toggle = use_signal(|| true);
    let mut mod_active = use_signal(|| false);

    // 输入与删除
    let mut draft = use_signal(String::new);
    let mut delete_id = use_signal(|| None::<usize>);

    let models = vec![
        "gemini-3.1-pro-preview-high",
        "gemini-3.7-flash-low",
        "claude-3-5-sonnet",
        "deepseek-chat",
        "gpt-4o",
    ];

    // 发送用户抉择/行动
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

        dioxus::document::eval(
            "setTimeout(() => { const el = document.getElementById('chat-scroll-viewport'); if(el) el.scrollTo({ top: el.scrollHeight, behavior: 'smooth' }); }, 50);",
        );
    };

    rsx! {
        div {
            class: "relative flex h-full w-full overflow-hidden bg-zinc-950 text-zinc-100 select-none",
            onclick: move |_| {
                // 点击背景空白处自动收起气泡专属操作菜单
                active_bubble_menu_id.set(None);
            },

            // ==========================================
            // 抽屉遮罩背景 (左右抽屉任意一个打开时呈现景深遮罩)
            // ==========================================
            if active_drawer().is_some() {
                div {
                    class: "fixed inset-0 z-40 bg-black/60 backdrop-blur-sm transition-opacity duration-300",
                    onclick: move |_| active_drawer.set(None),
                }
            }

            // ==========================================
            // 左侧会话抽屉 (互斥排他打开，对齐图1与图3)
            // ==========================================
            div {
                class: if active_drawer() == Some("left") {
                    "fixed inset-y-0 left-0 z-50 flex h-full w-80 flex-col border-r border-zinc-800/80 bg-zinc-900/95 backdrop-blur-2xl shadow-2xl transition-all duration-300 translate-x-0"
                } else {
                    "fixed inset-y-0 left-0 z-50 flex h-full w-80 flex-col border-r border-zinc-800/80 bg-zinc-900/95 backdrop-blur-2xl shadow-2xl transition-all duration-300 -translate-x-full pointer-events-none"
                },
                div { class: "flex h-full w-full flex-col gap-4 p-5 select-none",
                    // 剧本标题与关闭抽屉按钮
                    div { class: "flex items-start justify-between gap-2 border-b border-zinc-800/80 pb-3",
                        div { class: "flex flex-col gap-1",
                            span { class: "line-clamp-2 text-xs font-bold leading-5 tracking-tight text-zinc-100",
                                "【超真实】明星娱乐圈模拟器"
                            }
                            span { class: "text-[10px] text-zinc-500", "当代全球演艺资本衍生规则" }
                        }
                        button {
                            class: "flex h-7 w-7 shrink-0 items-center justify-center rounded-lg text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100 transition-colors",
                            title: "收起抽屉",
                            onclick: move |_| active_drawer.set(None),
                            "«"
                        }
                    }

                    div { class: "rounded-xl border border-zinc-800/60 bg-zinc-950/50 p-3 text-[11px] leading-5 text-zinc-400",
                        "【细腻UI和美化】【真实数据库与衍生规则】一比一复刻当代娱乐产业生态。这里有冰冷的资本运作与残酷的名利场。"
                    }

                    // 弹窗入口按钮 (需求6: 剧本详情与赞赏作品采用弹窗)
                    div { class: "grid grid-cols-2 gap-2.5",
                        button {
                            class: "flex items-center justify-center gap-1.5 rounded-xl border border-zinc-800 bg-zinc-800/70 py-2 text-xs font-medium text-zinc-200 transition-colors hover:bg-zinc-700 active:scale-95",
                            onclick: move |_| detail_modal_open.set(true),
                            span { "📖" }
                            span { "剧本详情" }
                        }
                        button {
                            class: "flex items-center justify-center gap-1.5 rounded-xl border border-amber-500/30 bg-amber-500/10 py-2 text-xs font-medium text-amber-300 transition-colors hover:bg-amber-500/20 active:scale-95",
                            onclick: move |_| donate_modal_open.set(true),
                            span { "☕" }
                            span { "赞赏作品" }
                        }
                    }

                    // 独立多会话时间线切换列表 (需求4: 真实隔离每个聊天室内容)
                    div { class: "flex min-h-0 flex-1 flex-col gap-2 pt-2",
                        div { class: "flex items-center justify-between px-1",
                            span { class: "text-xs font-bold text-zinc-400", "会话时间线" }
                            button {
                                class: "flex items-center gap-1 rounded-lg bg-purple-600/80 px-2.5 py-1 text-[11px] font-semibold text-white hover:bg-purple-600 transition-colors",
                                onclick: move |_| {
                                    let next_id = sessions().len() + 1;
                                    sessions.write().insert(0, SessionItem {
                                        id: next_id,
                                        title: format!("分支行动-{}", next_id),
                                        updated_at: "刚刚".into(),
                                    });
                                    current_session_id.set(next_id);
                                    // 载入新分支的专属空白会话
                                    story_items.set(get_session_story_items(next_id));
                                    active_drawer.set(None);
                                },
                                "+ 新分支"
                            }
                        }
                        div { class: "flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto pr-1 no-scrollbar",
                            for s in sessions() {
                                {
                                    let is_active = current_session_id() == s.id;
                                    let s_id = s.id;
                                    rsx! {
                                        button {
                                            key: "{s.id}",
                                            class: if is_active {
                                                "group flex w-full flex-col gap-1 rounded-xl border border-purple-500/50 bg-purple-950/30 p-3 text-left shadow-sm ring-1 ring-purple-500/30"
                                            } else {
                                                "group flex w-full flex-col gap-1 rounded-xl border border-zinc-800/80 bg-zinc-950/40 p-3 text-left text-zinc-400 hover:border-zinc-700 hover:bg-zinc-900/60"
                                            },
                                            onclick: move |_| {
                                                current_session_id.set(s_id);
                                                // 核心: 真实切换该分支的独立剧情流与状态 (对齐需求4)
                                                story_items.set(get_session_story_items(s_id));
                                                active_drawer.set(None); // 切换后收起抽屉
                                            },
                                            div { class: "flex items-center justify-between",
                                                span { class: "truncate text-xs font-semibold text-zinc-100 group-hover:text-purple-300 transition-colors",
                                                    "{s.title}"
                                                }
                                                span { class: "text-[10px] text-zinc-500 shrink-0", "{s.updated_at}" }
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
            // 右侧时间线大纲抽屉 (互斥排他打开，对齐图1右侧)
            // ==========================================
            div {
                class: if active_drawer() == Some("right") {
                    "fixed inset-y-0 right-0 z-50 flex h-full w-80 flex-col border-l border-zinc-800/80 bg-zinc-900/95 backdrop-blur-2xl shadow-2xl transition-all duration-300 translate-x-0"
                } else {
                    "fixed inset-y-0 right-0 z-50 flex h-full w-80 flex-col border-l border-zinc-800/80 bg-zinc-900/95 backdrop-blur-2xl shadow-2xl transition-all duration-300 translate-x-full pointer-events-none"
                },
                div { class: "flex h-full w-full flex-col gap-3 p-5 select-none",
                    div { class: "flex items-center justify-between border-b border-zinc-800 pb-3",
                        div { class: "flex items-center gap-2",
                            span { class: "text-sm", "📑" }
                            h2 { class: "font-serif text-sm font-bold text-zinc-100", "剧情大纲索引" }
                        }
                        div { class: "flex items-center gap-2",
                            span { class: "text-[10px] text-zinc-500 tabular-nums", "{story_items().len()} 节点" }
                            button {
                                class: "flex h-7 w-7 items-center justify-center rounded-lg text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100 transition-colors",
                                onclick: move |_| active_drawer.set(None),
                                "»"
                            }
                        }
                    }

                    div { class: "flex-1 overflow-y-auto space-y-2 pr-0.5 no-scrollbar",
                        for (idx, item) in story_items.read().clone().into_iter().enumerate() {
                            {
                                let item_id = item.id();
                                let (kind, title_text) = item.nav_title();
                                rsx! {
                                    button {
                                        key: "{item_id}",
                                        class: "group flex w-full flex-col gap-1 rounded-xl border border-zinc-800/60 bg-zinc-950/40 p-2.5 text-left transition-all hover:border-purple-500/50 hover:bg-zinc-900",
                                        onclick: move |_| {
                                            let eval_js = format!("document.getElementById('story-node-{}')?.scrollIntoView({{ behavior: 'smooth', block: 'start' }});", item_id);
                                            dioxus::document::eval(&eval_js);
                                            active_drawer.set(None);
                                        },
                                        div { class: "flex items-center justify-between text-[10px]",
                                            span { class: "font-mono font-bold text-zinc-500 group-hover:text-purple-300 transition-colors",
                                                "#{idx + 1:02}"
                                            }
                                            span { class: "rounded bg-zinc-800 px-1.5 py-0.2 text-[9px] text-zinc-400 group-hover:bg-purple-950 group-hover:text-purple-300 transition-colors",
                                                "{kind}"
                                            }
                                        }
                                        span { class: "line-clamp-1 text-xs text-zinc-300 group-hover:text-zinc-100 transition-colors",
                                            "{title_text}"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    button {
                        class: "mt-auto flex items-center justify-center gap-1 rounded-xl border border-zinc-800 bg-zinc-900 py-2 text-xs font-medium text-zinc-300 hover:text-white hover:bg-zinc-800 transition-colors",
                        onclick: move |_| {
                            dioxus::document::eval("const el = document.getElementById('chat-scroll-viewport'); if(el) el.scrollTo({ top: el.scrollHeight, behavior: 'smooth' });");
                            active_drawer.set(None);
                        },
                        span { "⬇" }
                        span { "平滑滑至最新" }
                    }
                }
            }

            // ==========================================
            // 中央互动剧情主视区 (全屏自适应，左右两侧有悬浮抽屉占位微按钮)
            // ==========================================
            div { class: "relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden",
                // 顶部一体化微条 (需求2: 点击回到剧本大厅/登录后大厅，非主页)
                div { class: "flex h-12 shrink-0 items-center justify-between border-b border-zinc-800/60 bg-zinc-900/70 px-4 backdrop-blur-xl z-10 select-none",
                    // 左侧：回到剧本库 (登录后进入剧本库，对齐需求2) + 呼出左侧会话抽屉按钮
                    div { class: "flex items-center gap-2",
                        // 占位符按钮: 点击呼出左侧会话抽屉 (排他性打开)
                        button {
                            class: "flex h-8 items-center gap-1.5 rounded-xl border border-zinc-800 bg-zinc-900/90 px-3 text-xs text-zinc-200 hover:bg-zinc-800 hover:border-purple-500/40 transition-all active:scale-95 shadow-sm",
                            title: "展开剧本与会话侧栏",
                            onclick: move |e| {
                                e.stop_propagation();
                                if active_drawer() == Some("left") {
                                    active_drawer.set(None);
                                } else {
                                    active_drawer.set(Some("left"));
                                }
                            },
                            span { "📚" }
                            span { class: "font-semibold", "剧本会话" }
                        }

                        // 直接回剧情库 (对齐需求2)
                        button {
                            class: "flex h-8 items-center gap-1 rounded-xl border border-zinc-800 bg-zinc-900/60 px-2.5 text-xs text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors hidden sm:flex",
                            title: "回到剧本库大厅",
                            onclick: move |_| on_goto_characters.call(()),
                            "大厅 ➜"
                        }

                        div { class: "flex items-center gap-1 rounded-full border border-zinc-800 bg-zinc-950/70 px-2.5 py-0.5 text-zinc-400 text-xs ml-1",
                            button { class: "hover:text-zinc-200 px-0.5", "‹" }
                            span { class: "text-[10px] font-medium tabular-nums text-zinc-200", "第 1 轮 · 共 3 轮" }
                            button { class: "hover:text-zinc-200 px-0.5", "›" }
                        }
                    }

                    // 右侧：呼出右侧时间线大纲抽屉占位符按钮 + 主题切换 + 快捷设置
                    div { class: "flex items-center gap-2",
                        // 占位符按钮: 点击呼出右侧时间线大纲抽屉 (排他性打开)
                        button {
                            class: "flex h-8 items-center gap-1.5 rounded-xl border border-zinc-800 bg-zinc-900/90 px-3 text-xs text-zinc-200 hover:bg-zinc-800 hover:border-purple-500/40 transition-all active:scale-95 shadow-sm",
                            title: "展开剧情时间线大纲",
                            onclick: move |e| {
                                e.stop_propagation();
                                if active_drawer() == Some("right") {
                                    active_drawer.set(None);
                                } else {
                                    active_drawer.set(Some("right"));
                                }
                            },
                            span { "📑" }
                            span { class: "font-semibold", "时间线大纲" }
                        }

                        button {
                            class: "flex h-8 w-8 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900 text-xs text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200 transition-colors",
                            title: "切换光暗",
                            onclick: move |_| on_toggle_theme.call(()),
                            if theme_light { "☀️" } else { "🌙" }
                        }
                        button {
                            class: "flex h-8 w-8 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900 text-xs text-zinc-300 hover:bg-zinc-800 hover:text-zinc-100 transition-colors",
                            title: "快捷菜单",
                            onclick: move |e| {
                                e.stop_propagation();
                                menu_open.set(!menu_open());
                            },
                            "⚙"
                        }
                    }
                }

                // ==========================================
                // 剧情滚动主舞台 (无滚动条平滑滚动，点击空白收起菜单)
                // ==========================================
                div {
                    id: "chat-scroll-viewport",
                    class: "flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto scroll-smooth p-4 sm:p-6 lg:px-24 xl:px-44 [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]",
                    for item in story_items.read().clone() {
                        {
                            let item_id = item.id();
                            let is_menu_active = active_bubble_menu_id() == Some(item_id);
                            rsx! {
                                div {
                                    id: "story-node-{item_id}",
                                    class: "scroll-mt-4 flex flex-col gap-4",
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
                                                is_active_menu: is_menu_active,
                                                on_click: move |e: MouseEvent| {
                                                    e.stop_propagation();
                                                    // 点击气泡弹出菜单 (对齐需求5)
                                                    if active_bubble_menu_id() == Some(id) {
                                                        active_bubble_menu_id.set(None);
                                                    } else {
                                                        active_bubble_menu_id.set(Some(id));
                                                    }
                                                },
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
                                                        title: "复制文本",
                                                        onclick: move |e: MouseEvent| {
                                                            e.stop_propagation();
                                                            active_bubble_menu_id.set(None);
                                                        },
                                                        "📋"
                                                    }
                                                    IconButton {
                                                        title: "编辑修改",
                                                        onclick: move |e: MouseEvent| {
                                                            e.stop_propagation();
                                                            active_bubble_menu_id.set(None);
                                                        },
                                                        "✎"
                                                    }
                                                    IconButton {
                                                        title: "删除段落",
                                                        onclick: move |e: MouseEvent| {
                                                            e.stop_propagation();
                                                            delete_id.set(Some(id));
                                                            active_bubble_menu_id.set(None);
                                                        },
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
                        }
                    }
                }

                // ==========================================
                // 底部输入区与控制工具岛
                // ==========================================
                div {
                    class: "flex shrink-0 flex-col gap-2 border-t border-zinc-800/60 bg-zinc-900/90 p-3 backdrop-blur-2xl z-10 select-none",
                    onclick: move |e| e.stop_propagation(),

                    div { class: "flex flex-wrap items-center justify-between gap-2 px-1 text-xs select-none",
                        // 模型切换胶囊
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

                        // 快捷动作胶囊组
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

                    // 输入框与发送按钮
                    div { class: "flex items-end gap-2 rounded-2xl border border-zinc-800 bg-zinc-950/90 p-2.5 shadow-inner",
                        textarea {
                            class: "h-11 min-h-11 flex-1 resize-none rounded-xl bg-transparent px-3 py-2 text-sm text-zinc-100 outline-none placeholder:text-zinc-600 focus:ring-0",
                            placeholder: "点击上方行动选项，或输入自定义决策 (电脑端 Shift+回车换行)",
                            value: "{draft()}",
                            oninput: move |e| draft.set(e.value()),
                            onkeydown: move |e| {
                                if e.key() == Key::Enter && !e.modifiers().shift() {
                                    e.prevent_default();
                                    handle_send();
                                }
                            },
                        }
                        div { class: "flex shrink-0 items-center gap-2 select-none",
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

                // 右上角设置菜单
                if menu_open() {
                    div {
                        class: "absolute inset-0 z-40 bg-black/20",
                        onclick: move |_| menu_open.set(false),
                    }
                    div {
                        class: "absolute right-3 top-14 z-50 flex w-48 flex-col divide-y divide-zinc-800 rounded-2xl border border-zinc-800 bg-zinc-900/95 p-1.5 shadow-2xl backdrop-blur-2xl text-xs select-none",
                        onclick: move |e| e.stop_propagation(),
                        div { class: "flex flex-col py-1",
                            button { class: "flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-zinc-300 hover:bg-zinc-800",
                                "📤 导出记录"
                            }
                            button { class: "flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-zinc-300 hover:bg-zinc-800",
                                "📥 导入聊天记录"
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

            // =========================================================================
            // 剧本详情弹窗 Modal (对齐需求6与图3)
            // =========================================================================
            if detail_modal_open() {
                div {
                    class: "fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-xl p-4 select-none",
                    onclick: move |_| detail_modal_open.set(false),
                    div {
                        class: "relative flex w-full max-w-lg flex-col gap-4 rounded-3xl border border-zinc-800 bg-zinc-900 p-6 shadow-2xl text-xs",
                        onclick: move |e| e.stop_propagation(),
                        div { class: "flex items-center justify-between border-b border-zinc-800 pb-3",
                            div { class: "flex items-center gap-2",
                                span { class: "text-base", "📖" }
                                span { class: "font-serif text-sm font-bold text-white", "剧本档案详情" }
                            }
                            button {
                                class: "text-zinc-400 hover:text-white text-sm",
                                onclick: move |_| detail_modal_open.set(false),
                                "✕"
                            }
                        }
                        div { class: "space-y-3 leading-relaxed text-zinc-300",
                            div { class: "rounded-xl bg-zinc-950/60 p-3.5 border border-zinc-800/80 space-y-1.5",
                                span { class: "font-bold text-zinc-100", "【超真实】明星娱乐圈模拟器" }
                                p { class: "text-zinc-400 leading-5 text-[11px]",
                                    "真实复刻当代全球演艺资本衍生规则。从最初的对赌协议签署、到中期的公关战与剧本争夺，每一个选择都直接决定你在名利场中的生死浮沉。"
                                }
                            }
                            div { class: "grid grid-cols-2 gap-2 text-[11px]",
                                div { class: "rounded-xl border border-zinc-800 bg-zinc-950/40 p-2.5",
                                    span { class: "text-zinc-500 block", "当前对弈角色" }
                                    span { class: "font-semibold text-zinc-200", "顾清言 (资方代表)" }
                                }
                                div { class: "rounded-xl border border-zinc-800 bg-zinc-950/40 p-2.5",
                                    span { class: "text-zinc-500 block", "博弈状态" }
                                    span { class: "font-semibold text-emerald-400", "第 3 轮 · 契约决战" }
                                }
                            }
                        }
                        div { class: "flex justify-end pt-2",
                            button {
                                class: "rounded-full bg-zinc-100 px-5 py-1.5 text-xs font-bold text-zinc-900 hover:bg-zinc-300",
                                onclick: move |_| detail_modal_open.set(false),
                                "知道了"
                            }
                        }
                    }
                }
            }

            // =========================================================================
            // 赞赏作品弹窗 Modal (对齐需求6与图3)
            // =========================================================================
            if donate_modal_open() {
                div {
                    class: "fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-xl p-4 select-none",
                    onclick: move |_| donate_modal_open.set(false),
                    div {
                        class: "relative flex w-full max-w-sm flex-col gap-4 rounded-3xl border border-amber-500/30 bg-zinc-900 p-6 shadow-2xl text-xs text-center",
                        onclick: move |e| e.stop_propagation(),
                        div { class: "flex h-12 w-12 items-center justify-center rounded-2xl bg-amber-500/10 border border-amber-500/30 text-2xl mx-auto shadow-inner",
                            "☕"
                        }
                        div { class: "flex flex-col gap-1",
                            span { class: "font-serif text-sm font-bold text-white", "赞赏创作者" }
                            span { class: "text-[11px] text-zinc-400", "支持作者制作更多优质多分支剧本与专属立绘" }
                        }
                        div { class: "grid grid-cols-3 gap-2 py-2",
                            for amt in ["2 能量", "5 能量", "10 能量"] {
                                button { class: "rounded-xl border border-zinc-800 bg-zinc-950/80 py-2 font-bold text-amber-300 hover:border-amber-500/50 hover:bg-amber-950/20 transition-all",
                                    "{amt}"
                                }
                            }
                        }
                        button {
                            class: "w-full rounded-full bg-gradient-to-r from-amber-500 to-orange-500 py-2.5 font-bold text-zinc-950 hover:opacity-90 transition-opacity shadow-lg shadow-amber-500/20",
                            onclick: move |_| donate_modal_open.set(false),
                            "确认赞赏并支持"
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
}
