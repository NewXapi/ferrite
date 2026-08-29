//! Auth page private form components.

use dioxus::prelude::*;
use ui::{CodeField, Field, SubmitButton};

#[component]
pub fn TabButton(label: String, selected: bool, onclick: EventHandler<MouseEvent>) -> Element {
    let tone = if selected {
        "border-zinc-100 text-zinc-100"
    } else {
        "border-transparent text-zinc-500 hover:text-zinc-300 hover:border-zinc-600"
    };
    rsx! {
        button {
            class: "relative -mb-px rounded-t-lg border-b-2 px-4 py-2 text-sm font-medium transition-colors {tone}",
            r#type: "button",
            onclick: move |event| onclick.call(event),
            "{label}"
        }
    }
}

#[component]
pub fn SignInForm() -> Element {
    rsx! {
        form { class: "space-y-5",
            Field {
                label: "Username or email",
                name: "username",
                r#type: "text",
                placeholder: "name@example.com",
            }
            Field {
                label: "Password",
                name: "password",
                r#type: "password",
                placeholder: "••••••••",
            }
            div { class: "flex items-center justify-between text-sm text-zinc-400",
                label { class: "flex items-center gap-2",
                    input { class: "size-4 rounded border-zinc-700 bg-zinc-900", r#type: "checkbox" }
                    "Remember me"
                }
                a { class: "text-zinc-400 hover:text-zinc-100 transition-colors", href: "#", "Forgot password?" }
            }
            SubmitButton { label: "Sign in" }
        }
    }
}

#[component]
pub fn SignUpForm() -> Element {
    rsx! {
        form { class: "space-y-5",
            Field {
                label: "Username",
                name: "username",
                r#type: "text",
                placeholder: "Enter your username",
            }
            Field {
                label: "Email",
                name: "email",
                r#type: "email",
                placeholder: "name@example.com",
            }
            CodeField {
                label: "Verification code".to_string(),
                name: "verification_code".to_string(),
                placeholder: "6-digit code".to_string(),
            }
            Field {
                label: "Password",
                name: "password",
                r#type: "password",
                placeholder: "8-20 characters",
            }
            Field {
                label: "Confirm password",
                name: "confirm_password",
                r#type: "password",
                placeholder: "Repeat password",
            }
            SubmitButton { label: "Create account" }
        }
    }
}
