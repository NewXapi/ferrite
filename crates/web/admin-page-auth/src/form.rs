//! Auth page private form components: field styles + real submit wiring.

use dioxus::prelude::*;
use ui::{CodeField, FormField as Field, SubmitButton};

/// 提交共享状态：错误信息 + busy（按钮禁用）。
#[derive(Clone, Default)]
pub struct SubmitState {
    pub error: Signal<Option<String>>,
    pub busy: Signal<bool>,
}

#[component]
fn SubmitStateBanner(error: Signal<Option<String>>) -> Element {
    match error.read().as_deref() {
        Some(e) if !e.is_empty() => rsx! {
            div {
                class: "rounded-lg border border-red-500/40 bg-red-500/10 px-3 py-2 text-xs text-red-400",
                "{e}"
            }
        },
        _ => rsx! {},
    }
}

#[derive(Clone)]
pub struct SignInPayload {
    pub username: String,
    pub password: String,
}

#[component]
pub fn SignInForm(submit: EventHandler<SignInPayload>) -> Element {
    let error = use_context::<SubmitState>().error;
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);

    rsx! {
        form {
            class: "space-y-5",
            onsubmit: move |ev| {
                ev.prevent_default();
                submit.call(SignInPayload {
                    username: username.read().clone(),
                    password: password.read().clone(),
                });
            },
            Field {
                label: "Username or email",
                name: "username",
                placeholder: "name@example.com",
                value: username(),
                oninput: move |ev: dioxus::prelude::FormEvent| username.set(ev.value()),
            }
            Field {
                label: "Password",
                name: "password",
                r#type: "password",
                placeholder: "••••••••",
                value: password(),
                oninput: move |ev: dioxus::prelude::FormEvent| password.set(ev.value()),
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
            SubmitStateBanner { error }
            div {
                class: "pt-2",
                SubmitButton { label: "Sign in" }
            }
        }
    }
}

#[derive(Clone)]
pub struct SignUpPayload {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[component]
pub fn SignUpForm(submit: EventHandler<SignUpPayload>) -> Element {
    let mut error = use_context::<SubmitState>().error;
    let mut username = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut code = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut confirm = use_signal(String::new);

    rsx! {
        form {
            class: "space-y-5",
            onsubmit: move |ev| {
                ev.prevent_default();
                if password.read().as_str() != confirm.read().as_str() {
                    error.set(Some("Passwords do not match".into()));
                    return;
                }
                submit.call(SignUpPayload {
                    username: username.read().clone(),
                    email: email.read().clone(),
                    password: password.read().clone(),
                });
            },
            Field {
                label: "Username",
                name: "username",
                placeholder: "Choose a username",
                value: username(),
                oninput: move |ev: dioxus::prelude::FormEvent| username.set(ev.value()),
            }
            Field {
                label: "Email",
                name: "email",
                r#type: "email",
                placeholder: "name@example.com",
                value: email(),
                oninput: move |ev: dioxus::prelude::FormEvent| email.set(ev.value()),
            }
            CodeField {
                label: "Verification code",
                name: "verification_code",
                placeholder: "6-digit code",
                value: code(),
                oninput: move |ev: dioxus::prelude::FormEvent| code.set(ev.value()),
                on_send: move |_| {},
            }
            Field {
                label: "Password",
                name: "password",
                r#type: "password",
                placeholder: "8-20 characters",
                value: password(),
                oninput: move |ev: dioxus::prelude::FormEvent| password.set(ev.value()),
            }
            Field {
                label: "Confirm password",
                name: "confirm_password",
                r#type: "password",
                placeholder: "Repeat password",
                value: confirm(),
                oninput: move |ev: dioxus::prelude::FormEvent| confirm.set(ev.value()),
            }
            SubmitStateBanner { error }
            div {
                class: "pt-2",
                SubmitButton { label: "Create account" }
            }
        }
    }
}
