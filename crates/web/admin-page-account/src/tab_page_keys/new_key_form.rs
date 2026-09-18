//! 新建密钥弹窗 — 提交走 POST /api/token。
//! 额度输入留空 = 不限额 (unlimited_quota)。
//! 成功后由父组件弹出一次性明文视图。

use contract::api::token::{CreateTokenRequest, CreateTokenResult};
use dioxus::prelude::*;
use ui::components::button::{Button, ButtonSize, ButtonVariant};

use crate::api;

#[component]
pub fn NewKeyForm(
    name: Signal<String>,
    group: Signal<String>,
    quota: Signal<String>,
    on_cancel: EventHandler<()>,
    on_created: EventHandler<CreateTokenResult>,
) -> Element {
    let mut busy = use_signal(|| false);
    let mut err = use_signal(String::new);

    let submit = move |_| {
        let n = name().trim().to_string();
        if n.is_empty() {
            err.set("密钥名称必填".into());
            return;
        }
        let g = group().trim().to_string();
        let q_raw = quota().trim().to_string();
        let (quota_v, unlimited) = if q_raw.is_empty() {
            (0, true)
        } else {
            (q_raw.parse::<i64>().unwrap_or(0), false)
        };
        let req = CreateTokenRequest {
            name: n,
            group: if g.is_empty() { None } else { Some(g) },
            quota: quota_v,
            unlimited_quota: unlimited,
            // 新建暂不设置过期 (MVP); 需要时由 CreateTokenRequest.expires_at 传入 RFC3339。
            expires_at: None,
        };

        busy.set(true);
        err.set(String::new());
        let client = client::ApiClient::shared().clone();
        let mut b = busy;
        let mut er = err;
        spawn(async move {
            match api::create_token_api(&client, &req).await {
                Ok(res) => on_created.call(res),
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
                class: "w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                div { class: "mb-5 flex items-center justify-between",
                    h3 { class: "text-base font-semibold text-zinc-100", "新建 API 密钥" }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                        onclick: move |_| on_cancel.call(()),
                        "aria-label": "关闭",
                        "✕"
                    }
                }

                div { class: "space-y-4",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "密钥名称" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm focus:border-zinc-500 focus:outline-none",
                            placeholder: "例如: 生产环境密钥",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "分组 (可选)" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm focus:border-zinc-500 focus:outline-none",
                            placeholder: "留空 = 跟随用户默认分组",
                            value: "{group}",
                            oninput: move |e| group.set(e.value()),
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "额度限制 (额度单位)" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 font-mono text-sm focus:border-zinc-500 focus:outline-none",
                            r#type: "text",
                            placeholder: "留空 = 不限额",
                            value: "{quota}",
                            oninput: move |e| quota.set(e.value()),
                        }
                    }

                    if !err().is_empty() {
                        p { class: "text-xs text-red-400", "{err()}" }
                    }
                }

                div { class: "mt-6 flex gap-3",
                    Button {
                        variant: ButtonVariant::Outline,
                        class: "flex-1",
                        onclick: move |_| on_cancel.call(()),
                        "取消"
                    }
                    Button {
                        variant: ButtonVariant::Primary,
                        class: "flex-1",
                        disabled: busy(),
                        onclick: submit,
                        "创建密钥"
                    }
                }
            }
        }
    }
}