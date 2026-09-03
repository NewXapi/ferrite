//! tavern-page-chat — 聊天主界面。
//!
//! 当前为壳内效果页：消息放本地信号，发送只追加用户消息；
//! 流式生成、swipe、落盘待 client/state 会话接线。

use dioxus::prelude::*;

/// 本地消息：角色名 + 内容。`mine` 区分用户 / 角色气泡样式。
#[derive(Clone, PartialEq)]
struct ChatMessage {
    name: String,
    content: String,
    mine: bool,
}

#[component]
pub fn ChatPage() -> Element {
    let mut messages = use_signal(Vec::<ChatMessage>::new);
    let mut draft = use_signal(String::new);

    let mut send = move || {
        let text = draft().trim().to_string();
        if text.is_empty() {
            return;
        }
        messages.write().push(ChatMessage {
            name: "我".into(),
            content: text,
            mine: true,
        });
        draft.set(String::new());
    };

    rsx! {
        div { class: "flex h-full flex-col gap-3",
            div { class: "flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto rounded-xl border border-zinc-800 bg-zinc-950/40 p-4",
                if messages().is_empty() {
                    div { class: "flex flex-1 flex-col items-center justify-center gap-2 text-center",
                        span { class: "text-sm font-medium text-zinc-300", "还没有消息" }
                        span { class: "text-xs text-zinc-500", "在下方输入开始对话；流式生成接入后此处显示角色回复" }
                    }
                } else {
                    for (i, m) in messages().iter().enumerate() {
                        MessageBubble {
                            key: "{i}",
                            name: m.name.clone(),
                            content: m.content.clone(),
                            mine: m.mine,
                        }
                    }
                }
            }
            div { class: "flex shrink-0 items-end gap-2 rounded-xl border border-zinc-800 bg-zinc-900/60 p-2",
                textarea {
                    class: "h-10 min-h-10 flex-1 resize-none rounded-lg bg-transparent px-2 py-2 text-sm text-zinc-100 outline-none placeholder:text-zinc-600",
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
                    class: "rounded-full bg-zinc-100 px-4 py-2 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-300 disabled:cursor-not-allowed disabled:opacity-40",
                    disabled: draft().trim().is_empty(),
                    onclick: move |_| send(),
                    "发送"
                }
            }
        }
    }
}

#[component]
fn MessageBubble(name: String, content: String, mine: bool) -> Element {
    let align = if mine { "items-end" } else { "items-start" };
    let bubble = if mine {
        "bg-zinc-100 text-zinc-900"
    } else {
        "bg-zinc-800 text-zinc-100"
    };
    rsx! {
        div { class: "flex flex-col gap-1 {align}",
            span { class: "px-1 text-xs text-zinc-500", "{name}" }
            div { class: "max-w-[75%] whitespace-pre-wrap rounded-2xl px-3 py-2 text-sm leading-6 {bubble}",
                "{content}"
            }
        }
    }
}
