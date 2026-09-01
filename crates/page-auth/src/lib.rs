//! Auth page library: provides state context provider plus public entry.

use dioxus::prelude::*;

pub mod api;
pub mod form;
pub mod state;
pub mod view;

#[component]
pub fn AuthPageRoot() -> Element {
    use_auth_tab();
    rsx! { view::AuthPage {} }
}

use state::use_auth_tab;
