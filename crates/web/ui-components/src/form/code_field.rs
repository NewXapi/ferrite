use dioxus::prelude::*;

use super::INPUT_CLASS;
/// 验证码输入字段 (带发送按钮)
#[component]
pub fn CodeField(
    label: String,
    #[props(default)] name: String,
    #[props(default)] placeholder: String,
    #[props(default)] value: String,
    oninput: Option<EventHandler<FormEvent>>,
    #[props(default)] on_send: EventHandler<()>,
) -> Element {
    rsx! {
        label { class: "block space-y-1.5",
            span { class: "block {crate::T_text_xs} {crate::T_font_medium} {crate::T_text_zinc_400} uppercase tracking-wide", "{label}" }
            div { class: "flex gap-2",
                input {
                    "data-testid": "{name}",
                    class: INPUT_CLASS,
                    name: "{name}",
                    placeholder: "{placeholder}",
                    r#type: "text",
                    autocomplete: "one-time-code",
                    value: "{value}",
                    oninput: move |ev| {
                        if let Some(h) = &oninput {
                            h.call(ev);
                        }
                    },
                }
                button {
                    "data-testid": "{name}-send",
                    class: "shrink-0 rounded-lg border {crate::T_border_zinc_700} bg-zinc-800/50 px-4 py-2.5 {crate::T_text_xs} {crate::T_font_medium} {crate::T_text_zinc_300} transition-all hover:{crate::T_border_zinc_500} hover:{crate::T_bg_zinc_800} hover:{crate::T_text_zinc_100} active:scale-95",
                    r#type: "button",
                    onclick: move |_| on_send.call(()),
                    "Send code"
                }
            }
        }
    }
}
