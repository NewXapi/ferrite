use dioxus::prelude::*;

use super::INPUT_CLASS;
/// 快速输入型字段 (自带统一 input 样式)
#[component]
pub fn FormField(
    label: String,
    #[props(default)] name: String,
    #[props(default = "text".to_string())] r#type: String,
    #[props(default)] placeholder: String,
    #[props(default)] value: String,
    oninput: Option<EventHandler<FormEvent>>,
) -> Element {
    rsx! {
        label { class: "block space-y-1.5",
            span { class: "block text-xs font-medium text-zinc-400 uppercase tracking-wide", "{label}" }
            input {
                "data-testid": "{name}",
                class: INPUT_CLASS,
                name: "{name}",
                placeholder: "{placeholder}",
                r#type: "{r#type}",
                value: "{value}",
                oninput: move |ev| {
                    if let Some(h) = &oninput {
                        h.call(ev);
                    }
                },
            }
        }
    }
}
