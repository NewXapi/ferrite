//! tavern-page-chat — 文游与角色扮演互动界面。
//!
//! 布局为四区 dock：左列（角色卡 + 会话时间线）/ 中央互动区 / 右列
//! （prompt 导航 + 模型与轮次），各区上下可分、可跨区拖面板、可收起
//! 为图标轨；dock 状态经 [`dock::DOCK`] 全局信号驱动并持久化到
//! localStorage（key `tavern-dock-layout`）。中央互动区、composer、
//! 消息流、Dialog 群行为不变。
//!
//! 数据源：从 mock 切换到 tavern_state/tavern-client 的真实状态。

pub mod dock;
pub mod dock_panels;
pub mod layout;

use dioxus::prelude::*;
use tavern_state::{STATE, abort, init, select_character, send};
use tavern_ui::{Dialog, IconButton, MessageBubble, SwipePicker};

// tavern_client::save_chat 供删除 Dialog 使用

/// 会话项
#[derive(Clone, PartialEq)]
pub struct SessionItem {
    /// 聊天文件名(不带扩展名)
    pub title: String,
}

/// 消息气泡用的展示字段；数据源就是 tavern_state 的 Message。
pub fn msg_display(msg: &tavern_state::Message) -> (String, bool, usize, Vec<String>) {
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

/// 中央互动区 editor 槽：消息流 + composer + Dialog 群（顶栏在页面层全宽渲染）。
///
/// 作为独立组件方便 DockFrame 以 Element 形式接收。
#[component]
pub fn EditorSlot(
    /// 发送输入框草稿。
    draft: Signal<String>,
    /// 气泡菜单激活 id。
    active_bubble_menu_id: Signal<Option<usize>>,
    /// 删除目标消息 idx。
    delete_id: Signal<Option<usize>>,
    /// Mod 开关。
    mod_active: Signal<bool>,
    /// 记忆增强开关。
    memory_boost: Signal<bool>,
    /// 流式开关。
    stream_toggle: Signal<bool>,
    /// 剧本详情弹窗开关。
    detail_modal_open: Signal<bool>,
    /// 赞赏作品弹窗开关。
    donate_modal_open: Signal<bool>,
    /// 快捷菜单开关。
    menu_open: Signal<bool>,
    /// 当前滚动消息索引。
    active_prompt: Signal<usize>,
    /// 滚动去抖。
    scroll_dirty: Signal<bool>,
    /// 滚动任务进行中。
    scroll_running: Signal<bool>,
    /// 模型下拉开关。
    model_dropdown_open: Signal<bool>,
    /// 发送回调。
    handle_send: EventHandler<()>,
    /// 滚动回调。
    on_scroll: EventHandler<ScrollEvent>,
) -> Element {
    let mut draft = draft;
    // 气泡菜单 id 只在下方 msg_rows 的行级副本（ammi/del_id）里写入，本体只读
    let active_bubble_menu_id = active_bubble_menu_id;
    let mut delete_id = delete_id;
    let mut mod_active = mod_active;
    let mut memory_boost = memory_boost;
    let mut stream_toggle = stream_toggle;
    let mut detail_modal_open = detail_modal_open;
    let mut donate_modal_open = donate_modal_open;
    let mut menu_open = menu_open;

    // 消息行独立构建为 Element，避免外层 rsx! 里 for + 嵌套 rsx! 的解析坑
    let msgs_v: Vec<tavern_state::Message> = STATE.with(|s| s.messages.clone());
    let msg_rows: Vec<Element> = msgs_v
        .iter()
        .enumerate()
        .map(|(idx, msg)| {
            let (content, mine, swipe_idx, swipes) = msg_display(msg);
            let is_menu_active = active_bubble_menu_id() == Some(idx);
            let name = msg.name.clone();
            let time = msg.send_date.clone();
            let mut ammi = active_bubble_menu_id;
            let mut del_id = delete_id;
            let swipe_actions = rsx! {
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
                        ammi.set(None);
                    },
                    "复制"
                }
                IconButton {
                    title: "删除段落",
                    onclick: move |e: MouseEvent| {
                        e.stop_propagation();
                        del_id.set(Some(idx));
                        ammi.set(None);
                    },
                    "删除"
                }
            };
            let row: Element = rsx! {
                div {
                    key: "msg-{idx}",
                    id: "story-node-{idx}",
                    class: "scroll-mt-4 flex flex-col gap-4",
                    MessageBubble {
                        name: name.clone(),
                        time: time.clone(),
                        content: content.clone(),
                        mine,
                        is_active_menu: is_menu_active,
                        on_click: move |e: MouseEvent| {
                            e.stop_propagation();
                            if ammi() == Some(idx) {
                                ammi.set(None);
                            } else {
                                ammi.set(Some(idx));
                            }
                        },
                        actions: swipe_actions,
                    }
                }
            };
            row
        })
        .collect();

    rsx! {
        div { class: "relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden",

            div {
                id: "chat-scroll-viewport",
                class: "flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto overflow-x-hidden scroll-smooth p-4 sm:p-6 no-scrollbar",
                onscroll: move |e| on_scroll.call(e),
                { msg_rows.iter() }
            }

            // composer：独立 flex 兄弟节点贴中央区底部（sticky-in-viewport 在
            // 消息少时会停在内容顶部而非视口底部），宽度随中央列
            div {
                class: "shrink-0 flex w-full flex-col gap-2 border-t border-purple-500/20 bg-zinc-950 p-3",
                onclick: move |e| e.stop_propagation(),
                div { class: "flex flex-wrap items-center gap-1.5",
                    button {
                        class: if mod_active() {
                            "rounded-full border border-purple-500/40 bg-purple-500/20 px-2.5 py-1 text-[11px] font-medium text-purple-200"
                        } else {
                            "rounded-full border border-zinc-800 bg-zinc-950/70 px-2.5 py-1 text-[11px] text-zinc-400 hover:text-zinc-200"
                        },
                        onclick: move |_| mod_active.set(!mod_active()),
                        "Mod"
                    }
                    button {
                        class: if memory_boost() {
                            "rounded-full border border-emerald-500/40 bg-emerald-500/20 px-2.5 py-1 text-[11px] font-medium text-emerald-200"
                        } else {
                            "rounded-full border border-zinc-800 bg-zinc-950/70 px-2.5 py-1 text-[11px] text-zinc-400 hover:text-zinc-200"
                        },
                        onclick: move |_| memory_boost.set(!memory_boost()),
                        "记忆"
                    }
                    button {
                        class: if stream_toggle() {
                            "rounded-full border border-cyan-500/40 bg-cyan-500/20 px-2.5 py-1 text-[11px] font-medium text-cyan-200"
                        } else {
                            "rounded-full border border-zinc-800 bg-zinc-950/70 px-2.5 py-1 text-[11px] text-zinc-400 hover:text-zinc-200"
                        },
                        onclick: move |_| stream_toggle.set(!stream_toggle()),
                        "流式"
                    }
                }
                div { class: "flex items-end gap-2",
                    textarea {
                        class: "h-11 min-h-11 flex-1 resize-none rounded-xl bg-transparent px-3 py-2 text-sm text-zinc-100 outline-none placeholder:text-zinc-600 focus:ring-0",
                        placeholder: "输入你的决策或行动 (电脑端 Shift+回车换行)",
                        value: "{draft()}",
                        oninput: move |e| draft.set(e.value()),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter && !e.modifiers().shift() {
                                e.prevent_default();
                                handle_send.call(());
                            }
                        },
                    }
                }
                div { class: "flex items-center justify-between gap-2",
                    div { class: "flex min-w-0 items-center gap-2 text-[10px]",
                        if STATE.with(|s| s.generating) {
                            span { class: "flex items-center gap-1 text-cyan-400",
                                "生成中..."
                                button {
                                    class: "text-zinc-500 hover:text-white",
                                    onclick: move |_| abort(),
                                    "停止"
                                }
                            }
                        }
                    }
                    div { class: "flex shrink-0 items-center gap-2",
                        button {
                            class: "flex h-9 items-center justify-center rounded-full bg-gradient-to-r from-purple-600 to-pink-600 px-5 text-xs font-bold text-white shadow-md shadow-purple-600/30 transition-all hover:scale-105 hover:shadow-purple-600/50 disabled:opacity-40",
                            disabled: draft().trim().is_empty() || STATE.with(|s| s.generating),
                            onclick: move |_| handle_send.call(()),
                            if STATE.with(|s| s.generating) { "停止" } else { "行动" }
                        }
                    }
                }
            }

                    div { class: "flex shrink-0 items-center gap-2 border-t border-zinc-800/60 bg-zinc-900/90 px-3 py-1.5 backdrop-blur-2xl z-10 select-none",
                        if STATE.with(|s| s.generating) {
                            div { class: "flex items-center gap-2 text-[10px] text-cyan-400",
                                span { "生成中..." }
                                button {
                                    class: "text-zinc-500 hover:text-white",
                                    onclick: move |_| abort(),
                                    "停止"
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
                    { STATE.with(|s| {
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
                    }) }
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
                            "导出记录"
                        }
                    }
                }
            }
        }
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
    // 页面初始化：加载设置与角色列表；有角色则自动选中第一个
    use_effect(move || {
        spawn(async move {
            init().await;
            let first = STATE.with(|s| s.characters.first().map(|c| c.file_name.clone()));
            if let Some(file_name) = first {
                select_character(file_name).await;
            }
        });
    });

    // 页面 UI 状态
    // 以下信号都在 EditorSlot / 面板子组件内经副本写入，本组件只读直传，无需 mut；
    // draft 与 active_bubble_menu_id 在本组件闭包里有写入，保留 mut。
    let detail_modal_open = use_signal(|| false);
    let donate_modal_open = use_signal(|| false);
    let mut menu_open = use_signal(|| false);
    let model_dropdown_open = use_signal(|| false);
    let memory_boost = use_signal(|| true);
    let stream_toggle = use_signal(|| true);
    let mod_active = use_signal(|| false);
    let mut draft = use_signal(String::new);
    let delete_id = use_signal(|| None::<usize>);
    let mut active_bubble_menu_id = use_signal(|| None::<usize>);

    // prompt 导航条相关状态（滚动联动高亮）
    let active_prompt = use_signal(|| 0usize);
    let scroll_dirty = use_signal(|| false);
    let scroll_running = use_signal(|| false);

    // 发送处理
    let mut handle_send = move || {
        let text = draft().trim().to_string();
        if text.is_empty() {
            return;
        }
        spawn(async move {
            let _ = send(text).await;
        });
        draft.set(String::new());
        dioxus::document::eval(
            "setTimeout(() => { const el = document.getElementById('chat-scroll-viewport'); if(el) el.scrollTo({ top: el.scrollHeight, behavior: 'smooth' }); }, 50);",
        );
    };

    // 滚动联动 prompt 导航条：dirty/running 双信号去抖，避免每次 scroll 都起 eval 任务
    let on_scroll = {
        let mut active_prompt = active_prompt;
        let mut scroll_dirty = scroll_dirty;
        let mut scroll_running = scroll_running;
        move |_evt: ScrollEvent| {
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
                    if let Ok(value) = dioxus::document::eval(js).await
                        && let Some(n) = value.as_i64().or_else(|| value.as_u64().map(|x| x as i64))
                        && n >= 0
                    {
                        active_prompt.set(n as usize);
                    }
                    if !scroll_dirty() {
                        break;
                    }
                }
                scroll_running.set(false);
            });
        }
    };

    let handle_send_ev = Callback::new(move |_| handle_send());
    let on_scroll_ev = Callback::new(on_scroll);

    rsx! {
        div {
            class: "relative flex h-full w-full flex-col overflow-hidden bg-zinc-950 text-zinc-100 select-none",
            onclick: move |_| {
                active_bubble_menu_id.set(None);
            },

            // 顶层 header：全宽，dock 三列都在它下面
            div { class: "flex h-12 shrink-0 items-center justify-between border-b border-zinc-800/60 bg-zinc-900/70 px-4 backdrop-blur-xl z-20 select-none",
                div { class: "flex items-center gap-2",
                    button {
                        class: "flex h-8 items-center gap-1.5 rounded-xl border border-zinc-800 bg-zinc-900/90 px-3 text-xs text-zinc-200 hover:bg-zinc-800 hover:border-purple-500/40 transition-all active:scale-95 shadow-sm",
                        title: "剧本会话",
                        name: "btn-sidebar-toggle-top",
                        onclick: move |e| e.stop_propagation(),
                        span { class: "font-semibold", "剧本会话" }
                    }
                    button {
                        class: "flex h-8 items-center gap-1 rounded-xl border border-zinc-800 bg-zinc-900/60 px-2.5 text-xs text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors hidden sm:flex",
                        title: "回到剧本库大厅",
                        onclick: move |_| on_goto_characters.call(()),
                        "大厅"
                    }
                }
                div { class: "flex items-center gap-2",
                    button {
                        class: "flex h-8 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900 text-xs text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200 transition-colors",
                        title: "切换光暗",
                        onclick: move |_| on_toggle_theme.call(()),
                        if theme_light { "暗" } else { "亮" }
                    }
                    button {
                        class: "flex h-8 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900 px-3 text-xs text-zinc-300 hover:bg-zinc-800 hover:text-zinc-100 transition-colors",
                        title: "快捷菜单",
                        onclick: move |e| {
                            e.stop_propagation();
                            menu_open.set(!menu_open());
                        },
                        "菜单"
                    }
                }
            }

            // 三列 dock 区（顶栏之下）
            div { class: "flex min-h-0 flex-1",
                layout::DockFrame {
                editor: rsx! {
                    EditorSlot {
                        draft: draft,
                        active_bubble_menu_id: active_bubble_menu_id,
                        delete_id: delete_id,
                        mod_active: mod_active,
                        memory_boost: memory_boost,
                        stream_toggle: stream_toggle,
                        detail_modal_open: detail_modal_open,
                        donate_modal_open: donate_modal_open,
                        menu_open: menu_open,
                        active_prompt: active_prompt,
                        scroll_dirty: scroll_dirty,
                        scroll_running: scroll_running,
                        model_dropdown_open: model_dropdown_open,
                        handle_send: handle_send_ev,
                        on_scroll: on_scroll_ev,
                    }
                },
                // 四个面板元素按默认停靠区直传：实际落区由 dock::DOCK 信号在
                // layout::render_zone 中动态决定，prop 名只对应默认区，不锁定面板位置。
                left_top: rsx! {
                    dock_panels::CharacterPanel {
                        detail_modal_open: detail_modal_open,
                        donate_modal_open: donate_modal_open,
                    }
                },
                left_bottom: rsx! {
                    dock_panels::SessionsPanel {}
                },
                right_top: rsx! {
                    dock_panels::PromptPanel {
                        active_prompt: active_prompt,
                        hovered_prompt: None,
                    }
                },
                right_bottom: rsx! {
                    dock_panels::ModelPanel {
                        model_dropdown_open: model_dropdown_open,
                    }
                },
                }
            }
        }
    }
}
