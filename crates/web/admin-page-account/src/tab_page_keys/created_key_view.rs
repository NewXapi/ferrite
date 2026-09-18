//! 新建成功视图 — 一次性明文 key 展示 (后端只在创建响应返回一次)。
//! 提供一键复制明文 (这是唯一值得复制的完整密钥), 关闭后无法再查看。

use contract::api::token::CreateTokenResult;
use dioxus::prelude::*;
use ui::components::button::{Button, ButtonVariant};

#[component]
pub fn CreatedKeyView(result: CreateTokenResult, on_close: EventHandler<()>) -> Element {
    let mut copied = use_signal(|| false);
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            div {
                class: "w-full max-w-md rounded-2xl border border-emerald-500/40 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                div { class: "mb-4 flex items-center justify-between",
                    h3 { class: "text-base font-semibold text-emerald-400", "密钥创建成功" }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                        onclick: move |_| on_close.call(()),
                        "aria-label": "关闭",
                        "✕"
                    }
                }

                p { class: "mb-2 text-xs text-amber-400", "明文密钥只显示这一次,关闭后无法再查看" }
                div { class: "flex items-center gap-2",
                    input {
                        class: "min-w-0 flex-1 rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-3 font-mono text-sm text-emerald-300 focus:outline-none",
                        r#type: "text",
                        r#readonly: true,
                        value: "{result.plaintext}"
                    }
                    button {
                        class: if copied() {
                            "shrink-0 rounded-xl border border-emerald-500/40 bg-emerald-500/10 px-3 py-3 text-xs font-medium text-emerald-400"
                        } else {
                            "shrink-0 rounded-xl border border-zinc-700 bg-zinc-950 px-3 py-3 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-800"
                        },
                        "aria-label": "复制明文密钥",
                        onclick: move |_| {
                            // fire-and-forget 写入剪贴板, 提交即视为发起成功
                            let ok = ui::copy_text_to_clipboard(&result.plaintext);
                            copied.set(ok);
                            let mut c = copied;
                            spawn(async move {
                                gloo_timers::future::TimeoutFuture::new(1500).await;
                                c.set(false);
                            });
                        },
                        if copied() { "已复制" } else { "复制" }
                    }
                }
                p { class: "mt-2 text-xs text-zinc-500", "名称: {result.token.name}" }

                div { class: "mt-5 flex justify-end",
                    Button {
                        variant: ButtonVariant::Primary,
                        onclick: move |_| on_close.call(()),
                        "完成"
                    }
                }
            }
        }
    }
}