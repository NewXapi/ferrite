//! tavern-page-settings — 连接与采样设置页。
//!
//! 当前为壳内效果页：表单值放本地信号，保存仅在内存生效；
//! `/tavern/settings`、`/tavern/secrets`、`/v1/models` 待 client 会话接线。

use dioxus::prelude::*;

#[component]
pub fn SettingsPage() -> Element {
    let mut model = use_signal(String::new);
    let mut api_key = use_signal(String::new);
    let mut temperature = use_signal(|| 0.7_f64);
    let mut top_p = use_signal(|| 0.9_f64);
    let mut max_tokens = use_signal(|| 512_u32);
    let mut saved = use_signal(|| false);

    rsx! {
        div { class: "flex h-full flex-col gap-6 overflow-y-auto",
            section { class: "flex flex-col gap-3",
                div { class: "flex items-center gap-2",
                    span { class: "text-sm font-medium text-zinc-200", "连接" }
                    span { class: "rounded-full bg-zinc-800 px-2 py-0.5 text-xs text-zinc-500",
                        "密钥状态：未知"
                    }
                }
                Field { label: "模型名",
                    input {
                        class: "w-full rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-sm text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-zinc-600",
                        placeholder: "如 gpt-4o-mini",
                        value: "{model()}",
                        oninput: move |e| {
                            model.set(e.value());
                            saved.set(false);
                        },
                    }
                }
                Field { label: "API Key",
                    input {
                        r#type: "password",
                        class: "w-full rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-sm text-zinc-100 outline-none placeholder:text-zinc-600 focus:border-zinc-600",
                        placeholder: "sk-…",
                        value: "{api_key()}",
                        oninput: move |e| {
                            api_key.set(e.value());
                            saved.set(false);
                        },
                    }
                }
            }
            section { class: "flex flex-col gap-3",
                span { class: "text-sm font-medium text-zinc-200", "采样" }
                SliderField {
                    label: "temperature",
                    value: temperature(),
                    min: 0.0,
                    max: 2.0,
                    step: 0.05,
                    on_change: move |v| {
                        temperature.set(v);
                        saved.set(false);
                    },
                }
                SliderField {
                    label: "top_p",
                    value: top_p(),
                    min: 0.0,
                    max: 1.0,
                    step: 0.01,
                    on_change: move |v| {
                        top_p.set(v);
                        saved.set(false);
                    },
                }
                Field { label: "max_tokens",
                    input {
                        r#type: "number",
                        class: "w-32 rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-sm text-zinc-100 outline-none focus:border-zinc-600",
                        value: "{max_tokens()}",
                        oninput: move |e| {
                            if let Ok(v) = e.value().parse() {
                                max_tokens.set(v);
                                saved.set(false);
                            }
                        },
                    }
                }
            }
            div { class: "mt-auto flex items-center gap-3 border-t border-zinc-800 pt-4",
                button {
                    class: "rounded-full bg-zinc-100 px-4 py-1.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-300",
                    onclick: move |_| saved.set(true),
                    "保存"
                }
                if saved() {
                    span { class: "text-xs text-zinc-500", "已保存到本地状态；接口接线后写入 /tavern/settings" }
                }
            }
        }
    }
}

#[component]
fn Field(label: &'static str, children: Element) -> Element {
    rsx! {
        label { class: "flex flex-col gap-1.5",
            span { class: "text-xs font-medium text-zinc-400", "{label}" }
            {children}
        }
    }
}

#[component]
fn SliderField(
    label: &'static str,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    on_change: EventHandler<f64>,
) -> Element {
    rsx! {
        div { class: "flex items-center gap-3",
            span { class: "w-28 shrink-0 text-xs font-medium text-zinc-400", "{label}" }
            input {
                r#type: "range",
                class: "h-1 flex-1 accent-zinc-100",
                min: "{min}",
                max: "{max}",
                step: "{step}",
                value: "{value}",
                oninput: move |e| {
                    if let Ok(v) = e.value().parse() {
                        on_change.call(v);
                    }
                },
            }
            span { class: "w-12 shrink-0 text-right text-xs tabular-nums text-zinc-300", "{value:.2}" }
        }
    }
}
