//! Auth page private form components.
//! Minimal form styles for clean login experience.

use dioxus::prelude::*;
use ui::{CodeField, Field, SubmitButton};

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
            div {
                class: "flex items-center justify-between text-sm pt-1",
                label {
                    class: "flex items-center gap-2 cursor-pointer group",
                    input {
                        class: "size-4 rounded border-zinc-700 bg-zinc-800/60 text-zinc-100 transition-colors focus:ring-1 focus:ring-zinc-500 group-hover:border-zinc-600",
                        r#type: "checkbox"
                    }
                    span { class: "text-zinc-400 group-hover:text-zinc-300 transition-colors", "Remember me" }
                }
                a {
                    class: "text-zinc-400 hover:text-zinc-200 transition-colors hover:underline underline-offset-2",
                    href: "#",
                    "Forgot password?"
                }
            }
            div {
                class: "pt-2",
                SubmitButton { label: "Sign in" }
            }
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
                placeholder: "Choose a username",
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
            div {
                class: "pt-2",
                SubmitButton { label: "Create account" }
            }
        }
    }
}
