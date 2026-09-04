//! tavern-page-chat — 聊天主界面。
//!
//! 对齐 SillyTavern `#chat`:
//! - 角色上下文顶栏(头像+名字+状态点)
//! - 消息气泡(头像+时间戳+hover 浮出操作栏:编辑/删除/重生成)
//! - swipe 选择器 `‹ 1/2 ›`(角色多条回复分支切换)
//! - 行内消息编辑
//! - 确认删除弹窗
//! - 输入区(圆角大框+快捷发送)
//!
//! 当前为壳内效果页,内置一条示例对话呈现交互效果,待 client/state 接线。

use dioxus::prelude::*;
use tavern_ui::{Avatar, Dialog, IconButton, MessageBubble, SwipePicker};

/// 单条聊天消息。角色消息支持多段 swipe 历史。
#[derive(Clone, PartialEq)]
struct Message {
    id: usize,
    name: String,
    time: String,
    mine: bool,
    swipes: Vec<String>,
    swipe_idx: usize,
}

impl Message {
    fn content(&self) -> &str {
        self.swipes.get(self.swipe_idx).map(|s| s.as_str()).unwrap_or("")
    }
}

/// 演示用初始对话,展示 ST 的核心气泡、多 swipe、hover 效果。
fn seed_messages() -> Vec<Message> {
    vec![
        Message {
            id: 1,
            name: "巡音ルカ".into(),
            time: "14:20".into(),
            mine: false,
            swipes: vec![
                "今天录音结束得比较早……你有空吗？我知道一家安静的咖啡馆。".into(),
                "辛苦啦。今晚没有排练，要不要一起走走？".into(),
            ],
            swipe_idx: 0,
        },
        Message {
            id: 2,
            name: "我".into(),
            time: "14:22".into(),
            mine: true,
            swipes: vec!["好啊，正好想喝点热的。在哪碰头？".into()],
            swipe_idx: 0,
        },
        Message {
            id: 3,
            name: "巡音ルカ".into(),
            time: "14:23".into(),
            mine: false,
            swipes: vec!["演播厅出门往右走两分钟，那家蓝色招牌的店。我在门口等你。".into()],
            swipe_idx: 0,
        },
    ]
}

#[component]
pub fn ChatPage() -> Element {
    let mut messages = use_signal(seed_messages);
    let mut draft = use_signal(String::new);
    let mut editing_id = use_signal(|| None::<usize>);
    let mut edit_text = use_signal(String::new);
    let mut delete_id = use_signal(|| None::<usize>);

    let mut send = move || {
        let text = draft().trim().to_string();
        if text.is_empty() {
            return;
        }
        let next_id = messages().iter().map(|m| m.id).max().unwrap_or(0) + 1;
        messages.write().push(Message {
            id: next_id,
            name: "我".into(),
            time: "刚刚".into(),
            mine: true,
            swipes: vec![text],
            swipe_idx: 0,
        });
        draft.set(String::new());
    };

    rsx! {
        div { class: "flex h-full flex-col gap-3",
            // 角色会话顶栏
            div { class: "flex shrink-0 items-center justify-between rounded-xl border border-zinc-800/80 bg-zinc-900/40 px-3 py-2",
                div { class: "flex items-center gap-2.5",
                    Avatar { name: "巡音ルカ".to_string(), size: "h-7 w-7".to_string() }
                    div { class: "flex items-center gap-2",
                        span { class: "text-xs font-medium text-zinc-200", "巡音ルカ" }
                        span { class: "h-1.5 w-1.5 rounded-full bg-emerald-500", title: "在线" }
                    }
                }
                div { class: "flex items-center gap-2 text-xs text-zinc-500",
                    span { "共 {messages().len()} 条消息" }
                }
            }

            // 消息主滚动区
            div { class: "flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto rounded-xl border border-zinc-800 bg-zinc-950/40 p-4",
                if messages().is_empty() {
                    tavern_ui::EmptyState {
                        title: "还没有消息".to_string(),
                        hint: "在下方输入开始对话;流式生成接入后此处显示角色回复".to_string(),
                    }
                } else {
                    for m in messages.read().clone() {
                        if editing_id() == Some(m.id) {
                            div { class: "flex flex-col gap-2 rounded-2xl border border-zinc-700 bg-zinc-900 p-3",
                                textarea {
                                    class: "h-20 w-full resize-none rounded-lg bg-zinc-950 px-2 py-1.5 text-sm text-zinc-100 outline-none focus:ring-1 focus:ring-zinc-600",
                                    value: "{edit_text()}",
                                    oninput: move |e| edit_text.set(e.value()),
                                }
                                div { class: "flex justify-end gap-2",
                                    button {
                                        class: "rounded-full px-3 py-1 text-xs text-zinc-400 hover:text-zinc-100",
                                        onclick: move |_| editing_id.set(None),
                                        "取消"
                                    }
                                    button {
                                        class: "rounded-full bg-zinc-100 px-3 py-1 text-xs font-medium text-zinc-900 hover:bg-zinc-300",
                                        onclick: move |_| {
                                            if let Some(idx) = messages().iter().position(|x| x.id == m.id) {
                                                let t = edit_text();
                                                let mut ms = messages.write();
                                                let cur_swipe = ms[idx].swipe_idx;
                                                ms[idx].swipes[cur_swipe] = t;
                                            }
                                            editing_id.set(None);
                                        },
                                        "保存"
                                    }
                                }
                            }
                        } else {
                            MessageBubble {
                                key: "{m.id}",
                                name: m.name.clone(),
                                time: m.time.clone(),
                                content: m.content().to_string(),
                                mine: m.mine,
                                actions: rsx! {
                                    if !m.mine {
                                        SwipePicker {
                                            index: m.swipe_idx,
                                            total: m.swipes.len(),
                                            on_prev: move |_| {
                                                if let Some(idx) = messages().iter().position(|x| x.id == m.id) {
                                                    let mut ms = messages.write();
                                                    if ms[idx].swipe_idx > 0 {
                                                        ms[idx].swipe_idx -= 1;
                                                    }
                                                }
                                            },
                                            on_next: move |_| {
                                                if let Some(idx) = messages().iter().position(|x| x.id == m.id) {
                                                    let mut ms = messages.write();
                                                    if ms[idx].swipe_idx + 1 < ms[idx].swipes.len() {
                                                        ms[idx].swipe_idx += 1;
                                                    }
                                                }
                                            },
                                        }
                                    }
                                    IconButton {
                                        title: "编辑",
                                        onclick: {
                                            let content = m.content().to_string();
                                            move |_| {
                                                edit_text.set(content.clone());
                                                editing_id.set(Some(m.id));
                                            }
                                        },
                                        "✎"
                                    }
                                    IconButton {
                                        title: "删除",
                                    onclick: move |_| delete_id.set(Some(m.id)),
                                        "✕"
                                    }
                                },
                            }
                        }
                    }
                }
            }

            // ST 式输入框:圆角大框 + 发送按钮
            div { class: "flex shrink-0 items-end gap-2 rounded-2xl border border-zinc-800 bg-zinc-900/60 p-2.5 shadow-lg shadow-black/20",
                textarea {
                    class: "h-11 min-h-11 flex-1 resize-none rounded-xl bg-transparent px-3 py-2 text-sm text-zinc-100 outline-none placeholder:text-zinc-600 focus:bg-zinc-950/40",
                    placeholder: "输入消息，Enter 发送，Shift+Enter 换行",
                    value: "{draft()}",
                    oninput: move |e| draft.set(e.value()),
                    onkeydown: move |e| {
                        if e.key() == Key::Enter && !e.modifiers().shift() {
                            e.prevent_default();
                            send();
                        }
                    },
                }
                button {
                    class: "flex h-9 items-center justify-center rounded-full bg-zinc-100 px-4 text-xs font-semibold text-zinc-900 transition-all hover:bg-zinc-300 disabled:cursor-not-allowed disabled:opacity-40",
                    disabled: draft().trim().is_empty(),
                    onclick: move |_| send(),
                    "发送"
                }
            }
        }

        // 删除确认弹窗
        Dialog {
            title: "删除消息".to_string(),
            open: delete_id().is_some(),
            on_cancel: move |_| delete_id.set(None),
            on_confirm: move |_| {
                if let Some(id) = delete_id() {
                    messages.write().retain(|m| m.id != id);
                    delete_id.set(None);
                }
            },
            "确定要删除这条消息吗？此操作无法撤销。"
        }
    }
}
