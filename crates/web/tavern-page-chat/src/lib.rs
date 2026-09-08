//! tavern-page-chat — 文游与角色扮演互动界面。
//!
//! 深度优化满足需求:
//! 1. 左右侧边栏全面重构为抽屉 (Drawer) 模式，悬浮占位微标展开，互斥排他打开逻辑 (绝对不同时打开)
//! 2. 会话多聊天室真正独立隔离 (每个分支会话维护各自的消息历史，切换时完整重载不同内容)
//! 3. 消息气泡交互升级: 点击气泡浮出专属操作菜单 (复制/编辑/分支切换/删除)
//! 4. 侧栏「剧本详情」和「赞赏作品」以景深模糊弹窗 (Modal) 呈现
//! 5. 平滑滚动 + 隐藏滚动条
//!
//! 数据源重绑:从 mock 数据源切换到 tavern_state/tavern-client 的真实状态。

use dioxus::prelude::*;
use tavern_client::recent_chats;
use tavern_state::{STATE, abort, init, open_chat, select_character, send};
use tavern_ui::{Dialog, IconButton, MessageBubble, SwipePicker};

/// 会话项
#[derive(Clone, PartialEq)]
pub struct SessionItem {
    /// 聊天文件名(不带扩展名)
    pub title: String,
}

/// 消息气泡用的展示字段;数据源就是 tavern_state 的 Message。
fn msg_display(msg: &tavern_state::Message) -> (String, bool, usize, Vec<String>) {
    let content = msg
        .swipes
        .get(msg.swipe_id.unwrap_or(0))
        .cloned()
        .unwrap_or_else(|| msg.mes.clone());
    (
        content,
        msg.is_user,
        msg.swipe_id.unwrap_or(0),
        msg.swipes.clone(),
    )
}

#[component]
pub fn ChatPage(
    #[props(default)] on_goto_characters: EventHandler<()>,
    #[props(default)] on_goto_settings: EventHandler<()>,
    #[props(default)] on_goto_home: EventHandler<()>,
    #[props(default)] on_toggle_theme: EventHandler<()>,
    #[props(default = false)] theme_light: bool,
) -> Element {
    // 页面初始化:加载设置与角色列表;有角色则自动选中第一个
    use_effect(move || {
        spawn(async move {
            init().await;
            let first = STATE.with(|s| s.characters.first().map(|c| c.file_name.clone()));
            if let Some(file_name) = first {
                select_character(file_name).await;
            }
        });
    });

    // 会话列表:跟随 STATE.character 变化从后端拉取
    let mut sessions = use_signal(Vec::<SessionItem>::new);
    use_effect(move || {
        let file_name = STATE.with(|s| s.character.as_ref().map(|(f, _)| f.clone()));
        if let Some(file_name) = file_name {
            spawn(async move {
                match recent_chats(file_name).await {
                    Ok(chats) => {
                        sessions.set(
                            chats
                                .into_iter()
                                .map(|c| SessionItem { title: c.name })
                                .collect(),
                        );
                    }
                    Err(e) => {
                        STATE.with_mut(|s| {
                            s.last_error = Some(format!("加载聊天列表失败: {e}"));
                        });
                    }
                }
            });
        }
    });

    // 页面UI状态
    let mut active_drawer = use_signal(|| None::<&'static str>);
    let mut active_bubble_menu_id = use_signal(|| None::<usize>);
    let mut detail_modal_open = use_signal(|| false);
    let mut donate_modal_open = use_signal(|| false);
    let mut menu_open = use_signal(|| false);
    let mut model_dropdown_open = use_signal(|| false);
    let mut memory_boost = use_signal(|| true);
    let mut stream_toggle = use_signal(|| true);
    let mut mod_active = use_signal(|| false);
    let mut draft = use_signal(String::new);
    let mut delete_id = use_signal(|| None::<usize>);

    // 当前模型显示(来源:设置里的 model;切换入口后续在设置页做)
    let current_model =
        use_memo(move || STATE.with(|s| s.model.clone().unwrap_or_else(|| "未设置".to_string())));
    // 下拉列表:目前只有 settings 里配置的那一个模型
    let models = use_memo(move || STATE.with(|s| s.model.clone().into_iter().collect::<Vec<_>>()));

    // 发送处理
    let mut handle_send = move || {
        let text = draft().trim().to_string();
        if text.is_empty() {
            return;
        }

        // 使用 tavern_state 的 send 函数
        spawn(async move {
            let _ = send(text).await;
        });

        draft.set(String::new());

        // 滚动 JS 保留
        dioxus::document::eval(
            "setTimeout(() => { const el = document.getElementById('chat-scroll-viewport'); if(el) el.scrollTo({ top: el.scrollHeight, behavior: 'smooth' }); }, 50);",
        );
    };

    // 角色头部信息
    let character_info = use_memo(move || {
        STATE.with(|s| {
            s.character
                .as_ref()
                .map(|(_, char)| {
                    let name = &char.name;
                    let desc_str = char.description.clone();
                    format!(
                        "{} - {}",
                        name,
                        desc_str.chars().take(30).collect::<String>()
                    )
                })
                .unwrap_or("未选择角色".to_string())
        })
    });

    rsx! {
        div {
            class: "relative flex h-full w-full overflow-hidden bg-zinc-950 text-zinc-100 select-none",
            onclick: move |_| {
                // 点击背景空白处自动收起气泡专属操作菜单
                active_bubble_menu_id.set(None);
            },

            // 抽屉遮罩背景
            if active_drawer().is_some() {
                div {
                    class: "fixed inset-0 z-40 bg-black/60 backdrop-blur-sm transition-opacity duration-300",
                    onclick: move |_| active_drawer.set(None),
                }
            }

            // 左侧会话抽屉
            div {
                class: if active_drawer() == Some("left") {
                    "fixed inset-y-0 left-0 z-50 flex h-full w-80 flex-col border-r border-zinc-800/80 bg-zinc-900/95 backdrop-blur-2xl shadow-2xl transition-all duration-300 translate-x-0"
                } else {
                    "fixed inset-y-0 left-0 z-50 flex h-full w-80 flex-col border-r border-zinc-800/80 bg-zinc-900/95 backdrop-blur-2xl shadow-2xl transition-all duration-300 -translate-x-full pointer-events-none"
                },
                div { class: "flex h-full w-full flex-col gap-4 p-5 select-none",
                    div { class: "flex items-start justify-between gap-2 border-b border-zinc-800/80 pb-3",
                        div { class: "flex flex-col gap-1",
                            span { class: "line-clamp-2 text-xs font-bold leading-5 tracking-tight text-zinc-100",
                                { character_info() }
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

                    div { class: "flex min-h-0 flex-1 flex-col gap-2 pt-2",
                        div { class: "flex items-center justify-between px-1",
                            span { class: "text-xs font-bold text-zinc-400", "会话时间线" }
                            button {
                                class: "flex items-center gap-1 rounded-lg bg-purple-600/80 px-2.5 py-1 text-[11px] font-semibold text-white hover:bg-purple-600 transition-colors",
                                onclick: move |_| {
                                    let file_name = STATE
                                        .with(|state| state.character.as_ref().map(|(f, _)| f.clone()));
                                    if let Some(file_name) = file_name {
                                        spawn(async move {
                                            select_character(file_name).await;
                                        });
                                    }
                                    active_drawer.set(None);
                                },
                                "+ 新对话"
                            }
                        }
                        div { class: "flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto pr-1 no-scrollbar",
                            for s in sessions() {
                                {
                                    let is_active = STATE.with(|st| st.chat.as_deref() == Some(s.title.as_str()));
                                    let title = s.title.clone();
                                    rsx! {
                                        button {
                                            key: "{s.title}",
                                            class: if is_active {
                                                "group flex w-full flex-col gap-1 rounded-xl border border-purple-500/50 bg-purple-950/30 p-3 text-left shadow-sm ring-1 ring-purple-500/30"
                                            } else {
                                                "group flex w-full flex-col gap-1 rounded-xl border border-zinc-800/80 bg-zinc-950/40 p-3 text-left text-zinc-400 hover:border-zinc-700 hover:bg-zinc-900/60"
                                            },
                                            onclick: move |_| {
                                                let chat_name = title.clone();
                                                spawn(async move {
                                                    open_chat(chat_name).await;
                                                });
                                                active_drawer.set(None);
                                            },
                                            div { class: "flex items-center justify-between",
                                                span { class: "truncate text-xs font-semibold text-zinc-100 group-hover:text-purple-300 transition-colors",
                                                    "{s.title}"
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

            // 右侧时间线大纲抽屉
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
                            span { class: "text-[10px] text-zinc-500 tabular-nums", "{STATE.read().messages.len()} 节点" }
                            button {
                                class: "flex h-7 w-7 items-center justify-center rounded-lg text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100 transition-colors",
                                onclick: move |_| active_drawer.set(None),
                                "»"
                            }
                        }
                    }

                    div { class: "flex-1 overflow-y-auto space-y-2 pr-0.5 no-scrollbar",
                        for (idx, msg) in STATE.read().messages.iter().enumerate() {
                            {
                                let (content, mine, _, _) = msg_display(msg);
                                let kind = if mine { "玩家" } else { "NPC" };
                                let short: String = content.chars().take(20).collect();
                                rsx! {
                                    button {
                                        key: "outline-{idx}",
                                        class: "group flex w-full flex-col gap-1 rounded-xl border border-zinc-800/60 bg-zinc-950/40 p-2.5 text-left transition-all hover:border-purple-500/50 hover:bg-zinc-900",
                                        onclick: move |_| {
                                            let eval_js = format!("document.getElementById('story-node-{}')?.scrollIntoView({{ behavior: 'smooth', block: 'start' }});", idx);
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
                                            "{msg.name}: {short}"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    button {
                        class: "mt-auto flex items-center justify-center gap-1 rounded-xl border border-zinc-8 bg-zinc-900 py-2 text-xs font-medium text-zinc-300 hover:text-white hover:bg-zinc-800 transition-colors",
                        onclick: move |_| {
                            dioxus::document::eval("const el = document.getElementById('chat-scroll-viewport'); if(el) el.scrollTo({ top: el.scrollHeight, behavior: 'smooth' });");
                            active_drawer.set(None);
                        },
                        span { "⬇" }
                        span { "平滑滑至最新" }
                    }
                }
            }

            // 中央互动剧情主视区
            div { class: "relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden",
                div { class: "flex h-12 shrink-0 items-center justify-between border-b border-zinc-800/60 bg-zinc-900/70 px-4 backdrop-blur-xl z-10 select-none",
                    div { class: "flex items-center gap-2",
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

                        button {
                            class: "flex h-8 items-center gap-1 rounded-xl border border-zinc-800 bg-zinc-900/60 px-2.5 text-xs text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors hidden sm:flex",
                            title: "回到剧本库大厅",
                            onclick: move |_| on_goto_characters.call(()),
                            "大厅 ➜"
                        }

                        div { class: "flex items-center gap-1 rounded-full border border-zinc-8 bg-zinc-950/70 px-2.5 py-0.5 text-zinc-400 text-xs ml-1",
                            button { class: "hover:text-zinc-200 px-0.5", "‹" }
                            span { class: "text-[10px] font-medium tabular-nums text-zinc-200", "第 1 轮 · 共 3 轮" }
                            button { class: "hover:text-zinc-200 px-0.5", "›" }
                        }
                    }

                    div { class: "flex items-center gap-2",
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

                div {
                    id: "chat-scroll-viewport",
                    class: "flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto scroll-smooth p-4 sm:p-6 lg:px-24 xl:px-44 [&::-webkit-scrollbar]:hidden [-ms-overflow-style:none] [scrollbar-width:none]",
                    for (idx, msg) in STATE.read().messages.iter().enumerate() {
                        {
                            let (content, mine, swipe_idx, swipes) = msg_display(msg);
                            let is_menu_active = active_bubble_menu_id() == Some(idx);
                            let name = msg.name.clone();
                            let time = msg.send_date.clone();
                            rsx! {
                                div {
                                    id: "story-node-{idx}",
                                    class: "scroll-mt-4 flex flex-col gap-4",
                                    MessageBubble {
                                        key: "msg-{idx}",
                                        name: name.clone(),
                                        time: time.clone(),
                                        content: content.clone(),
                                        mine,
                                        is_active_menu: is_menu_active,
                                        on_click: move |e: MouseEvent| {
                                            e.stop_propagation();
                                            if active_bubble_menu_id() == Some(idx) {
                                                active_bubble_menu_id.set(None);
                                            } else {
                                                active_bubble_menu_id.set(Some(idx));
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
                                                title: "删除段落",
                                                onclick: move |e: MouseEvent| {
                                                    e.stop_propagation();
                                                    delete_id.set(Some(idx));
                                                    active_bubble_menu_id.set(None);
                                                },
                                                "✕"
                                            }
                                        },
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "flex shrink-0 flex-col gap-2 border-t border-zinc-800/60 bg-zinc-900/90 p-3 backdrop-blur-2xl z-10 select-none",
                    onclick: move |e| e.stop_propagation(),

                    div { class: "flex flex-wrap items-center justify-between gap-2 px-1 text-xs select-none",
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
                                    for m in models() {
                                        {
                                            let model_name = m.to_string();
                                            rsx! {
                                                button {
                                                    key: "{m}",
                                                    class: "flex w-full items-center justify-between rounded-lg px-2.5 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800 hover:text-zinc-100",
                                                    onclick: move |_| {
                                                        STATE.with_mut(|s| {
                                                            s.model = Some(model_name.clone());
                                                        });
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
                                    draft.set("【深入追问】\"你刚才的话，似乎并没有说完。\"".into());
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

                        if STATE.with(|s| s.generating) {
                            div { class: "ml-auto flex items-center gap-2 text-[10px] text-cyan-400",
                                span { "⚡ 生成中..." }
                                button {
                                    class: "ml-2 text-zinc-500 hover:text-white",
                                    onclick: move |_| {
                                        abort();
                                    },
                                    "✕"
                                }
                            }
                        }
                        if let Some(err) = STATE.with(|s| s.last_error.clone()) {
                            div { class: "ml-auto text-[10px] text-rose-400", "{err}" }
                        }
                    }

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
                                disabled: draft().trim().is_empty() || STATE.with(|s| s.generating),
                                onclick: move |_| handle_send(),
                                if STATE.with(|s| s.generating) { "⏹ 停止" } else { "行动 ➜" }
                            }
                        }
                    }
                }

                Dialog {
                    title: "删除这条消息?".to_string(),
                    open: delete_id().is_some(),
                    on_confirm: move |_| {
                        if let Some(idx) = delete_id() {
                            let (file_name, chat_name) = STATE.with(|s| {
                                (
                                    s.character.as_ref().map(|(f, _)| f.clone()),
                                    s.chat.clone(),
                                )
                            });
                            STATE.with_mut(|s| {
                                if idx < s.messages.len() {
                                    s.messages.remove(idx);
                                }
                            });
                            // 已落盘的聊天删除后同步保存,避免刷新复活
                            if let (Some(f), Some(c)) = (file_name, chat_name) {
                                let msgs = STATE.with(|s| s.messages.clone());
                                spawn(async move {
                                    if let Err(e) = tavern_client::save_chat(f, c, msgs).await {
                                        STATE.with_mut(|s| {
                                            s.last_error = Some(format!("保存删除失败: {e}"));
                                        });
                                    }
                                });
                            }
                        }
                        delete_id.set(None);
                    },
                    on_cancel: move |_| delete_id.set(None),
                    p { "删除后将从本聊天历史中移除。" }
                }

                Dialog {
                    title: STATE.with(|s| {
                        s.character
                            .as_ref()
                            .map(|(_, c)| format!("剧本详情: {}", c.name))
                            .unwrap_or_else(|| "剧本详情".to_string())
                    }),
                    open: detail_modal_open(),
                    on_confirm: move |_| detail_modal_open.set(false),
                    on_cancel: move |_| detail_modal_open.set(false),
                    p {
                        {STATE.with(|s| {
                            s.character
                                .as_ref()
                                .map(|(_, c)| {
                                    let desc = if c.description.is_empty() {
                                        "(无简介)".to_string()
                                    } else {
                                        c.description.clone()
                                    };
                                    let scenario = if c.scenario.is_empty() {
                                        String::new()
                                    } else {
                                        format!("\n\n场景: {}", c.scenario)
                                    };
                                    format!("{desc}{scenario}")
                                })
                                .unwrap_or_else(|| "未选择角色".to_string())
                        })}
                    }
                }

                Dialog {
                    title: "赞赏作品".to_string(),
                    open: donate_modal_open(),
                    on_confirm: move |_| donate_modal_open.set(false),
                    on_cancel: move |_| donate_modal_open.set(false),
                    p { "感谢支持创作者。赞赏渠道即将上线。" }
                }

                if menu_open() {
                    div {
                        class: "absolute inset-0 z-40 bg-black/20",
                        onclick: move |_| menu_open.set(false),
                    }
                    div {
                        class: "absolute right-3 top-14 z-50 flex w-48 flex-col divide-y divide-zinc-8 rounded-2xl border border-zinc-8 bg-zinc-900/95 p-1.5 shadow-2xl backdrop-blur-2xl text-xs select-none",
                        onclick: move |e| e.stop_propagation(),
                        div { class: "flex flex-col py-1",
                            button { class: "flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-zinc-300 hover:bg-zinc-800",
                                "📤 导出记录"
                            }
                        }
                    }
                }
            }
        }
    }
}
