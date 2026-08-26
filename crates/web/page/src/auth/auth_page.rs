use dioxus::prelude::*;

use super::fields::{CodeField, Field, SubmitButton};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AuthTab {
    SignIn,
    SignUp,
}

/// Shared auth page content (forms + tabs) for reuse in drawer or standalone.
#[component]
pub fn AuthPageContent() -> Element {
    let mut tab = use_signal(|| AuthTab::SignIn);
    let active = tab();

    rsx! {
        div {
            // Tab strip
            div { class: "mb-6 flex gap-1 border-b border-zinc-800",
                TabButton {
                    label: "Sign in",
                    selected: active == AuthTab::SignIn,
                    onclick: move |_| tab.set(AuthTab::SignIn),
                }
                TabButton {
                    label: "Register",
                    selected: active == AuthTab::SignUp,
                    onclick: move |_| tab.set(AuthTab::SignUp),
                }
            }

            // Forms
            match active {
                AuthTab::SignIn => rsx! { SignInForm {} },
                AuthTab::SignUp => rsx! { SignUpForm {} },
            }
        }
    }
}

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

/// Sign in form with username/email and password.
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

/// Sign up form with username, email, verification code, password, confirm.
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
                label: "Verification code",
                name: "verification_code",
                placeholder: "6-digit code",
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
