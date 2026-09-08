//! Auth page library: provides state context provider plus public entry.

use crate::form::SubmitState;
use dioxus::prelude::*;

pub mod api;
pub mod form;
pub mod state;
pub mod view;

#[component]
pub fn AuthPageRoot() -> Element {
    use_auth_tab();
    // SubmitState 在根组件 provide，保证所有 auth 子组件（form/view）可见
    use_context_provider(|| SubmitState {
        error: Signal::new(None),
        busy: Signal::new(false),
    });
    rsx! { view::AuthPage {} }
}

use state::use_auth_tab;
