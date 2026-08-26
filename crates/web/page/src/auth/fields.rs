use dioxus::prelude::*;

const INPUT_CLASS: &str = "w-full rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-3 text-zinc-100 outline-none transition focus:border-zinc-400 focus:ring-1 focus:ring-zinc-400";

/// Labeled text input used by the auth forms.
#[component]
pub fn Field(
    label: String,
    name: String,
    /// HTML input type, e.g. `text`, `password`, `email`.
    r#type: String,
    placeholder: String,
) -> Element {
    rsx! {
        label { class: "block space-y-2",
            span { class: "text-sm text-zinc-300", "{label}" }
            input{
                class: INPUT_CLASS,
                name: "{name}",
                placeholder: "{placeholder}",
                r#type: "{r#type}",
            }
        }
    }
}

/// Email verification code input paired with a "send code" action.
/// The code itself is issued by `GET /api/verification`.
#[component]
pub fn CodeField(label: String, name: String, placeholder: String) -> Element {
    rsx! {
        label { class: "block space-y-2",
            span { class: "text-sm text-zinc-300", "{label}" }
            div { class: "flex gap-2",
                input {
                    class: INPUT_CLASS,
                    name: "{name}",
                    placeholder: "{placeholder}",
                    r#type: "text",
                    autocomplete: "one-time-code",
                }
                button {
                    class: "shrink-0 rounded-xl border border-zinc-700 px-4 py-3 text-sm font-medium text-zinc-300 transition hover:border-zinc-500 hover:bg-zinc-800",
                    r#type: "button",
                    "Send code"
                }
            }
        }
    }
}

/// Full-width primary action button for the auth forms.
#[component]
pub fn SubmitButton(label: String) -> Element {
    rsx! {
        button {
            class: "w-full rounded-xl bg-zinc-100 px-4 py-3 font-semibold text-zinc-900 transition hover:bg-zinc-300",
            r#type: "button",
            "{label}"
        }
    }
}
