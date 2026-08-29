//! Auth page rendering and component composition.

use dioxus::prelude::*;
use crate::state::{AuthTab, auth_tab};
use crate::form::{SignInForm, SignUpForm, TabButton};

#[component]
pub fn AuthPage() -> Element {
    let active = auth_tab();

    rsx! {
        div {
            div { class: "mb-6 flex gap-1 border-b border-zinc-800",
                TabButton {
                    label: "Sign in",
                    selected: active == AuthTab::SignIn,
                    onclick: move |_| crate::state::set_auth_tab(AuthTab::SignIn),
                }
                TabButton {
                    label: "Register",
                    selected: active == AuthTab::SignUp,
                    onclick: move |_| crate::state::set_auth_tab(AuthTab::SignUp),
                }
            }
            match active {
                AuthTab::SignIn => rsx! { SignInForm {} },
                AuthTab::SignUp => rsx! { SignUpForm {} },
            }
        }
    }
}
