//! 删除确认弹窗 — 走 DELETE /api/token/{key}。

use contract::api::token::TokenDto;
use dioxus::prelude::*;
use ui::components::button::{Button, ButtonVariant};

use crate::api;

#[component]
pub fn DeleteKeyModal(
    token: TokenDto,
    on_cancel: EventHandler<()>,
    on_confirmed: EventHandler<()>,
) -> Element {
    let mut busy = use_signal(|| false);
    let mut err = use_signal(String::new);

    let confirm = move |_| {
        busy.set(true);
        err.set(String::new());
        let client = client::ApiClient::shared().clone();
        let key_id = token.key.clone();
        let mut b = busy;
        let mut er = err;
        spawn(async move {
            match api::delete_token_api(&client, &key_id).await {
                Ok(_) => on_confirmed.call(()),
                Err(e) => {
                    er.set(e.to_string());
                    b.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            onclick: move |_| on_cancel.call(()),
            div {
                class: "w-full max-w-md rounded-2xl border border-red-500/40 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                h3 { class: "text-base font-semibold text-zinc-100", "删除密钥" }
                p { class: "mt-3 text-sm text-zinc-400",
                    "确认删除「{token.name}」({token.key_preview})？删除后使用该密钥的调用会立即失败, 且无法恢复。"
                }
                if !err().is_empty() {
                    p { class: "mt-3 text-xs text-red-400", "{err()}" }
                }

                div { class: "mt-6 flex gap-3",
                    Button {
                        variant: ButtonVariant::Outline,
                        class: "flex-1",
                        onclick: move |_| on_cancel.call(()),
                        "取消"
                    }
                    Button {
                        variant: ButtonVariant::Destructive,
                        class: "flex-1",
                        disabled: busy(),
                        onclick: confirm,
                        "确认删除"
                    }
                }
            }
        }
    }
}
