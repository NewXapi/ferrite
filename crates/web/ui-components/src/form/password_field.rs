use dioxus::prelude::*;

use super::INPUT_CLASS;
/// 密码字段 (带显示/隐藏切换)
#[component]
pub fn PasswordField(
    label: String,
    #[props(default)] name: String,
    #[props(default)] placeholder: String,
    #[props(default)] value: String,
    oninput: Option<EventHandler<FormEvent>>,
) -> Element {
    let mut visible = use_signal(|| false);
    let eye = if visible() { "隐藏" } else { "显示" };
    rsx! {
        label { class: "block space-y-1.5",
            span { class: "block {crate::TYPE_DESC} uppercase tracking-wide", "{label}" }
            div { class: "relative", style: "position:relative; width:100%;",
                input {
                    class: "{INPUT_CLASS} pr-10",
                    name: "{name}",
                    placeholder: "{placeholder}",
                    r#type: if visible() { "text" } else { "password" },
                    value: "{value}",
                    oninput: move |ev| {
                        if let Some(h) = &oninput {
                            h.call(ev);
                        }
                    },
                }
                button {
                    class: "{crate::TYPE_BODY} hover:text-foreground transition-colors",
                    r#type: "button",
                    tabindex: "-1",
                    style: "position:absolute; right:12px; top:50%; transform:translateY(-50%);",
                    onclick: move |_| visible.toggle(),
                    aria_label: if visible() { "Hide password" } else { "Show password" },
                    "{eye}"
                }
            }
        }
    }
}
