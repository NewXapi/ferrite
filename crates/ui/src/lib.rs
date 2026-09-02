//! Shared form primitives. Field layout and verification-code action remain
//! visually compatible with the original auth page.

use dioxus::prelude::*;
mod scroll_spy;
mod segmented;

pub use scroll_spy::ScrollSpyNav;
pub use segmented::SegmentedCapsule;

const INPUT_CLASS: &str = "w-full rounded-lg border border-zinc-800 bg-zinc-900/80 px-3.5 py-2.5 text-sm text-zinc-100 placeholder-zinc-600 outline-none transition-all duration-200 hover:border-zinc-700 focus:border-zinc-500 focus:ring-2 focus:ring-zinc-500/20 focus:bg-zinc-900";

#[component]
pub fn Field(label: String, name: String, r#type: String, placeholder: String) -> Element {
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

/// Verification-code field with the original Send code action.
#[component]
pub fn CodeField(label: String, name: String, placeholder: String) -> Element {
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
                    "Send code"
                }
            }
        }
    }
}

#[component]
pub fn SubmitButton(label: String) -> Element {
    rsx! {
        button {
            class: "w-full rounded-lg bg-zinc-100 px-4 py-2.5 text-sm font-semibold text-zinc-900 transition-all duration-200 hover:bg-zinc-200 hover:shadow-lg hover:shadow-zinc-100/10 active:scale-[0.98] active:bg-zinc-300",
            r#type: "button",
            "{label}"
        }
    }
}
