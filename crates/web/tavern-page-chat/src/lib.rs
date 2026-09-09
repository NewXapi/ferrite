//! tavern-page-chat — 文游与角色扮演互动界面。
//!
//! 深度优化满足需求:
//! 1. 左侧改为常驻可收缩侧边栏 (Sidebar):展开显示角色/会话全信息,收缩为图标轨,会话以单字符号呈现;侧栏右侧再接一条 prompt 导航条 (按用户 prompt 数量生成,随滚动高亮当前 prompt,悬停预览完整 prompt)。原右侧「剧情大纲索引」浮层抽屉已移除。
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

/// 收缩态会话符号:chat-N -> 取序号 N(如 chat-4 -> "4"),其余取标题前 2 字。
/// 用作左侧侧栏图标轨上的单字徽标。
fn session_symbol(title: &str) -> String {
    if let Some(rest) = title.strip_prefix("chat-") {
        rest.chars().take(4).collect()
    } else {
        title.chars().take(2).collect()
    }
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
                                .map(|c| SessionItem { title: c.file_name })
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
    let mut active_drawer = use_signal(|| None::<&'static str>); // 仅右侧时间线大纲用浮层抽屉
    let mut sidebar_collapsed = use_signal(|| false); // 左侧常驻侧边栏:收缩/展开
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

    // 左侧 prompt 导航条相关状态
    let active_prompt = use_signal(|| 0usize); // 当前滚动到的消息索引(用于高亮对应 prompt)
    let scroll_dirty = use_signal(|| false); // 滚动去抖:仍有未处理的滚动
    let scroll_running = use_signal(|| false); // 滚动计算任务是否进行中
    let hovered_prompt = use_signal(|| None::<usize>); // 悬停预览的 prompt 序号

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

    // 滚动联动 prompt 导航条:根据视口顶部位置算出"当前消息",再映射到对应 prompt 高亮。
    // 用 dirty/running 双信号做去抖合并,避免每次 scroll 都起一个 eval 任务。
    let on_scroll = {
        let mut active_prompt = active_prompt;
        let mut scroll_dirty = scroll_dirty;
        let mut scroll_running = scroll_running;
        move |_evt| {
            scroll_dirty.set(true);
            if scroll_running() {
                return;
            }
            scroll_running.set(true);
            spawn(async move {
                loop {
                    scroll_dirty.set(false);
                    let js = r#"
                        (function(){
                            const vp = document.getElementById('chat-scroll-viewport');
                            if(!vp) return -1;
                            const vpr = vp.getBoundingClientRect();
                            const nodes = vp.querySelectorAll('[id^="story-node-"]');
                            let best = 0;
                            for (const n of nodes){
                                const rel = n.getBoundingClientRect().top - vpr.top;
                                if (rel <= 140) best = parseInt(n.id.split('-').pop(), 10);
                                else break;
                            }
                            return best;
                        })()
                    "#;
                    if let Ok(value) = dioxus::document::eval(js).await {
                        if let Some(n) = value.as_i64().or_else(|| value.as_u64().map(|x| x as i64)) {
                            if n >= 0 {
                                active_prompt.set(n as usize);
                            }
                        }
                    }
                    if !scroll_dirty() {
                        break;
                    }
                }
                scroll_running.set(false);
            });
        }
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

    // 角色单字徽标(收缩态头像用):取角色名首字
    let char_monogram = use_memo(move || {
        STATE.with(|s| {
            s.character
                .as_ref()
                .and_then(|(_, c)| c.name.chars().next())
                .map(|ch| ch.to_string())
                .unwrap_or_else(|| "?".to_string())
        })
    });

    rsx! {
        div {
            class: "relative flex h-full w-full overflow-hidden bg-zinc-950 text-zinc-100 select-none",
            onclick: move |_| {
                // 点击背景空白处自动收起气泡专属操作菜单
                active_bubble_menu_id.set(None);
            },

            // 抽屉遮罩背景(仅右侧时间线大纲用浮层)
            if active_drawer() == Some("right") {
                div {
                    class: "fixed inset-0 z-40 bg-black/60 backdrop-blur-sm transition-opacity duration-300",
                    onclick: move |_| active_drawer.set(None),
                }
            }

            // 左侧常驻侧边栏(可收缩:展开显全信息,收缩显图标轨与会话符号)
            div {
                class: if sidebar_collapsed() {
                    "flex h-full w-16 shrink-0 flex-col items-center gap-3 border-r border-zinc-800/60 bg-zinc-900/95 backdrop-blur-xl py-3 transition-all duration-300"
                } else {
                    "flex h-full w-72 shrink-0 flex-col border-r border-zinc-800/60 bg-zinc-900/95 backdrop-blur-xl transition-all duration-300"
                },
                "data-testid": "sidebar-characters",
                aria_label: "剧本与会话侧栏",

                // 顶栏:折叠/展开开关
                div { class: if sidebar_collapsed() {
                        "flex flex-col items-center"
                    } else {
                        "flex items-center justify-between gap-2 border-b border-zinc-800/80 px-3 pb-3 pt-3"
                    },
                    button {
                        class: "flex h-8 w-8 items-center justify-center rounded-lg text-zinc-400 hover:bg-zinc-800 hover:text-zinc-100 transition-colors",
                        title: if sidebar_collapsed() { "展开侧栏" } else { "收起侧栏" },
                        name: "btn-sidebar-toggle",
                        aria_label: if sidebar_collapsed() { "展开侧栏" } else { "收起侧栏" },
                        onclick: move |_| sidebar_collapsed.set(!sidebar_collapsed()),
                        if sidebar_collapsed() { "»" } else { "«" }
                    }
                    if !sidebar_collapsed() {
                        span { class: "truncate text-xs font-bold text-zinc-100", { character_info() } }
                    }
                }

                // 角色标识:收缩态显示单字徽标,展开态显示标签+简介
                if sidebar_collapsed() {
                    div { class: "flex h-9 w-9 items-center justify-center rounded-xl bg-gradient-to-br from-purple-600 to-pink-600 text-sm font-bold text-white shadow-md",
                        aria_label: "当前角色",
                        { char_monogram() }
                    }
                } else {
                    div { class: "flex flex-col gap-1 px-3 pt-2",
                        span { class: "text-[10px] text-zinc-500", "当代全球演艺资本衍生规则" }
                        div { class: "rounded-xl border border-zinc-800/60 bg-zinc-950/50 p-3 text-[11px] leading-5 text-zinc-400",
                            "【细腻UI和美化】【真实数据库与衍生规则】一比一复刻当代娱乐产业生态。这里有冰冷的资本运作与残酷的名利场。"
                        }
                    }
                }

                // 快捷按钮:剧本详情/赞赏(收缩态图标,展开态文字)
                if sidebar_collapsed() {
                    div { class: "flex flex-col items-center gap-2",
                        button {
                            class: "flex h-9 w-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-800/70 text-sm text-zinc-200 hover:bg-zinc-700 transition-colors",
                            title: "剧本详情",
                            name: "btn-detail",
                            onclick: move |_| detail_modal_open.set(true),
                            "📖"
                        }
                        button {
                            class: "flex h-9 w-9 items-center justify-center rounded-xl border border-amber-500/30 bg-amber-500/10 text-sm text-amber-300 hover:bg-amber-500/20 transition-colors",
                            title: "赞赏作品",
                            name: "btn-donate",
                            onclick: move |_| donate_modal_open.set(true),
                            "☕"
                        }
                    }
                } else {
                    div { class: "grid grid-cols-2 gap-2.5 px-3 pt-2",
                        button {
                            class: "flex items-center justify-center gap-1.5 rounded-xl border border-zinc-800 bg-zinc-800/70 py-2 text-xs font-medium text-zinc-200 transition-colors hover:bg-zinc-700 active:scale-95",
                            name: "btn-detail",
                            onclick: move |_| detail_modal_open.set(true),
                            span { "📖" }
                            span { "剧本详情" }
                        }
                        button {
                            class: "flex items-center justify-center gap-1.5 rounded-xl border border-amber-500/30 bg-amber-500/10 py-2 text-xs font-medium text-amber-300 transition-colors hover:bg-amber-500/20 active:scale-95",
                            name: "btn-donate",
                            onclick: move |_| donate_modal_open.set(true),
                            span { "☕" }
                            span { "赞赏作品" }
                        }
                    }
                }

                // 会话区
                div { class: if sidebar_collapsed() {
                        "flex min-h-0 flex-1 flex-col items-center gap-2 py-2 no-scrollbar"
                    } else {
                        "flex min-h-0 flex-1 flex-col gap-2 px-3 pt-2"
                    },
                    div { class: if sidebar_collapsed() {
                            "flex flex-col items-center gap-2"
                        } else {
                            "flex items-center justify-between px-1"
                        },
                        if !sidebar_collapsed() {
                            span { class: "text-xs font-bold text-zinc-400", "会话时间线" }
                        }
                        button {
                            class: if sidebar_collapsed() {
                                "flex h-9 w-9 items-center justify-center rounded-xl bg-purple-600/80 text-sm font-semibold text-white hover:bg-purple-600 transition-colors"
                            } else {
                                "flex items-center gap-1 rounded-lg bg-purple-600/80 px-2.5 py-1 text-[11px] font-semibold text-white hover:bg-purple-600 transition-colors"
                            },
                            title: "新对话",
                            name: "btn-new-chat",
                            onclick: move |_| {
                                let file_name = STATE
                                    .with(|state| state.character.as_ref().map(|(f, _)| f.clone()));
                                if let Some(file_name) = file_name {
                                    spawn(async move {
                                        select_character(file_name).await;
                                    });
                                }
                            },
                            if sidebar_collapsed() { "+" } else { "+ 新对话" }
                        }
                    }
                    div { class: if sidebar_collapsed() {
                            "flex min-h-0 flex-1 flex-col items-center gap-2 overflow-y-auto py-1 no-scrollbar"
                        } else {
                            "flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto pr-1 no-scrollbar"
                        },
                        for s in sessions() {
                            {
                                let is_active = STATE.with(|st| st.chat.as_deref() == Some(s.title.as_str()));
                                let title = s.title.clone();
                                let symbol = session_symbol(&s.title);
                                let btn_class = if sidebar_collapsed() {
                                    if is_active {
                                        "group flex h-9 w-9 items-center justify-center rounded-xl border border-purple-500/60 bg-purple-950/40 text-xs font-bold text-purple-100 ring-1 ring-purple-500/40 transition-colors"
                                    } else {
                                        "group flex h-9 w-9 items-center justify-center rounded-xl border border-zinc-800/70 bg-zinc-950/40 text-xs font-semibold text-zinc-400 hover:border-zinc-700 hover:bg-zinc-900/60 transition-colors"
                                    }
                                } else if is_active {
                                    "group flex w-full flex-col gap-1 rounded-xl border border-purple-500/50 bg-purple-950/30 p-3 text-left shadow-sm ring-1 ring-purple-500/30"
                                } else {
                                    "group flex w-full flex-col gap-1 rounded-xl border border-zinc-800/80 bg-zinc-950/40 p-3 text-left text-zinc-400 hover:border-zinc-700 hover:bg-zinc-900/60"
                                };
                                let display = if sidebar_collapsed() { symbol } else { s.title.clone() };
                                rsx! {
                                    button {
                                        key: "{s.title}",
                                        class: btn_class,
                                        title: "{s.title}",
                                        name: "session-{title}",
                                        aria_label: "会话 {title}",
                                        onclick: move |_| {
                                            let chat_name = title.clone();
                                            spawn(async move {
                                                open_chat(chat_name).await;
                                            });
                                        },
                                        "{display}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 左侧 prompt 导航条:按用户发送的 prompt 数量生成,当前滚动位置对应的 prompt 亮条高亮,悬停显示完整 prompt
            {
                let prompts: Vec<(usize, String, String)> = STATE.read().messages.iter().enumerate()
                    .filter_map(|(i, m)| {
                        let (content, mine, _, _) = msg_display(m);
                        if mine { Some((i, m.name.clone(), content)) } else { None }
                    }).collect();
                let active_msg = active_prompt();
                let active_prompt_idx = prompts.iter().rposition(|(mi, _, _)| *mi <= active_msg).unwrap_or(0);
                rsx! {
                    div { class: "relative flex w-9 shrink-0 flex-col bg-zinc-950/30 border-r border-zinc-800/40 select-none",
                        div { class: "flex h-full w-full flex-col items-center gap-1.5 overflow-y-auto py-3",
                            span { class: "text-[9px] font-bold tracking-widest text-zinc-600", "P" }
                            for (pi, (midx, name, content)) in prompts.iter().enumerate() {
                                {
                                    let is_active = active_prompt_idx == pi;
                                    let label = format!("{}", pi + 1);
                                    let name_c = name.clone();
                                    let content_c = content.clone();
                                    let midx_c = *midx;
                                    let tt = format!("{}: {}", name_c, content_c);
                                    let mut hp = hovered_prompt;
                                    rsx! {
                                        div {
                                            class: if is_active {
                                                "flex h-7 w-7 shrink-0 cursor-pointer items-center justify-center rounded-lg bg-purple-500 text-[11px] font-bold text-white shadow-[0_0_10px] shadow-purple-500/70 transition-all"
                                            } else {
                                                "flex h-7 w-7 shrink-0 cursor-pointer items-center justify-center rounded-lg bg-zinc-700/40 text-[11px] font-medium text-zinc-400 hover:bg-zinc-600/60 hover:text-zinc-100 transition-all"
                                            },
                                            "data-testid": format!("prompt-nav-{}", pi),
                                            title: "{tt}",
                                            aria_label: "prompt {label}",
                                            onmouseenter: move |_| hp.set(Some(pi)),
                                            onmouseleave: move |_| hp.set(None),
                                            onclick: move |_| {
                                                dioxus::document::eval(&format!("document.getElementById('story-node-{midx_c}')?.scrollIntoView({{ behavior: 'smooth', block: 'start' }});"));
                                            },
                                            "{label}"
                                        }
                                    }
                                }
                            }
                        }
                        if let Some(pi) = hovered_prompt() {
                            if let Some((_, name, content)) = prompts.get(pi) {
                                div { class: "pointer-events-none absolute left-full top-2 z-50 ml-1 w-60 rounded-xl border border-zinc-700/80 bg-zinc-900/95 p-2.5 text-[11px] leading-5 text-zinc-200 shadow-2xl backdrop-blur-xl",
                                    span { class: "mb-1 block text-[10px] font-bold text-purple-300", "{name}" }
                                    "{content}"
                                }
                            }
                        }
                    }
                }
            }

            // 中央互动剧情主视区
            div { class: "relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden",
                div { class: "flex h-12 shrink-0 items-center justify-between border-b border-zinc-800/60 bg-zinc-900/70 px-4 backdrop-blur-xl z-10 select-none",
                    div { class: "flex items-center gap-2",
                        button {
                            class: "flex h-8 items-center gap-1.5 rounded-xl border border-zinc-800 bg-zinc-900/90 px-3 text-xs text-zinc-200 hover:bg-zinc-800 hover:border-purple-500/40 transition-all active:scale-95 shadow-sm",
                            title: if sidebar_collapsed() { "展开剧本与会话侧栏" } else { "收起剧本与会话侧栏" },
                            name: "btn-sidebar-toggle-top",
                            onclick: move |e| {
                                e.stop_propagation();
                                sidebar_collapsed.set(!sidebar_collapsed());
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
                    onscroll: on_scroll,
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
