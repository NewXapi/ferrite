use dioxus::prelude::*;

pub const INPUT_CLASS: &str = "w-full rounded-lg border border-zinc-800 bg-zinc-900/80 px-3.5 py-2.5 text-sm text-zinc-100 placeholder-zinc-600 outline-none transition-all duration-200 hover:border-zinc-700 focus:border-zinc-500 focus:ring-2 focus:ring-zinc-500/20 focus:bg-zinc-900";

/// 容器型字段包装器 (包裹任意子控件 input / textarea)
#[component]
pub fn Field(label: &'static str, children: Element) -> Element {
    rsx! {
        label { class: "block space-y-1.5",
            span { class: "block text-xs font-medium text-zinc-400 uppercase tracking-wide", "{label}" }
            {children}
        }
    }
}

/// 快速输入型字段 (自带统一 input 样式)
#[component]
pub fn FormField(
    label: String,
    #[props(default)] name: String,
    #[props(default = "text".to_string())] r#type: String,
    #[props(default)] placeholder: String,
) -> Element {
    rsx! {
        label { class: "block space-y-1.5",
            span { class: "block text-xs font-medium text-zinc-400 uppercase tracking-wide", "{label}" }
            input {
                class: INPUT_CLASS,
                name: "{name}",
                placeholder: "{placeholder}",
                r#type: "{r#type}",
            }
        }
    }
}

/// 验证码输入字段 (带发送按钮)
#[component]
pub fn CodeField(
    label: String,
    #[props(default)] name: String,
    #[props(default)] placeholder: String,
    #[props(default)] on_send: EventHandler<()>,
) -> Element {
    rsx! {
        label { class: "block space-y-1.5",
            span { class: "block text-xs font-medium text-zinc-400 uppercase tracking-wide", "{label}" }
            div { class: "flex gap-2",
                input {
                    class: INPUT_CLASS,
                    name: "{name}",
                    placeholder: "{placeholder}",
                    r#type: "text",
                    autocomplete: "one-time-code",
                }
                button {
                    class: "shrink-0 rounded-lg border border-zinc-700 bg-zinc-800/50 px-4 py-2.5 text-xs font-medium text-zinc-300 transition-all hover:border-zinc-500 hover:bg-zinc-800 hover:text-zinc-100 active:scale-95",
                    r#type: "button",
                    onclick: move |_| on_send.call(()),
                    "Send code"
                }
            }
        }
    }
}

/// 表单提交按钮
#[component]
pub fn SubmitButton(label: String, #[props(default)] onclick: EventHandler<()>) -> Element {
    rsx! {
        button {
            class: "w-full rounded-lg bg-zinc-100 px-4 py-2.5 text-sm font-semibold text-zinc-900 transition-all duration-200 hover:bg-zinc-200 hover:shadow-lg hover:shadow-zinc-100/10 active:scale-[0.98] active:bg-zinc-300",
            r#type: "button",
            onclick: move |_| onclick.call(()),
            "{label}"
        }
    }
}

/// 滑杆字段 (对齐采样抽屉)
#[component]
pub fn SliderField(
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
