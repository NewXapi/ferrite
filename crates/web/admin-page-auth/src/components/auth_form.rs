//! Auth page private form components: field styles + real submit wiring.

use dioxus::prelude::*;
use ui::{FormField as Field, PasswordField, SubmitButton};

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
                class: "rounded-lg border border-destructive bg-destructive px-3 py-2 {ui::TYPE_DESC} {ui::C_DANGER}",
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
    pub remember: bool,
}

/// "记住我" 勾选框：登录 / 注册两表单共用的同一份标记。
#[component]
fn RememberMe(remember: Signal<bool>) -> Element {
    let box_class = if remember() {
        "bg-indigo-400 border-indigo-400"
    } else {
        "bg-secondary/60 border-border group-hover:border-zinc-400"
    };

    rsx! {
        label {
            class: "flex items-center gap-2 cursor-pointer group",
            div {
                class: "relative size-4 shrink-0 flex items-center justify-center rounded border transition-colors {box_class}",
                input {
                    style: "position:absolute; width:1px; height:1px; opacity:0; overflow:hidden;",
                    r#type: "checkbox",
                    checked: remember(),
                    oninput: move |ev| remember.set(ev.checked()),
                }
                if remember() {
                    span { class: "{ui::TYPE_LABEL} leading-none", "✓" }
                }
            }
            span { class: "{ui::C_MUTED} group-hover:text-foreground transition-colors", "Remember me" }
        }
    }
}

#[component]
pub fn SignInForm(submit: EventHandler<SignInPayload>, remember: Signal<bool>) -> Element {
    let error = use_context::<SubmitState>().error;
    let busy = use_context::<SubmitState>().busy;
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
                    remember: remember(),
                });
            },
            Field {
                label: "Username or email",
                name: "username",
                placeholder: "name@example.com",
                value: username(),
                oninput: move |ev: dioxus::prelude::FormEvent| username.set(ev.value()),
            }
            PasswordField {
                label: "Password",
                name: "password",
                placeholder: "••••••••",
                value: password(),
                oninput: move |ev: dioxus::prelude::FormEvent| password.set(ev.value()),
            }
            div {
                class: "flex items-center justify-between {ui::TYPE_BODY} pt-1",
                RememberMe { remember }
                span {
                    class: "cursor-pointer {ui::C_MUTED} hover:text-foreground transition-colors hover:underline underline-offset-2",
                    "Forgot password?"
                }
            }
            SubmitStateBanner { error }
            div {
                class: "pt-2",
                SubmitButton { label: "Sign in", busy: busy() }
            }
        }
    }
}

#[derive(Clone)]
pub struct SignUpPayload {
    pub username: String,
    pub email: String,
    pub password: String,
    pub remember: bool,
}

#[component]
pub fn SignUpForm(submit: EventHandler<SignUpPayload>, remember: Signal<bool>) -> Element {
    let busy = use_context::<SubmitState>().busy;
    let mut error = use_context::<SubmitState>().error;
    let mut username = use_signal(String::new);
    let mut email = use_signal(String::new);
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
                    remember: remember(),
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
                placeholder: "name@example.com (optional)",
                value: email(),
                oninput: move |ev: dioxus::prelude::FormEvent| email.set(ev.value()),
            }
            PasswordField {
                label: "Password",
                name: "password",
                placeholder: "8-20 characters",
                value: password(),
                oninput: move |ev: dioxus::prelude::FormEvent| password.set(ev.value()),
            }
            PasswordField {
                label: "Confirm password",
                name: "confirm_password",
                placeholder: "Repeat password",
                value: confirm(),
                oninput: move |ev: dioxus::prelude::FormEvent| confirm.set(ev.value()),
            }

            div {
                class: "flex items-center justify-between {ui::TYPE_BODY} pt-1",
                RememberMe { remember }
            }
            SubmitStateBanner { error }
            div {
                class: "pt-2",
                SubmitButton { label: "Create account", busy: busy() }
            }
        }
    }
}
