//! Auth page private state.

use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthTab {
    SignIn,
    SignUp,
}

pub fn auth_tab() -> AuthTab {
    use_context::<Signal<AuthTab>>()()
}

pub fn use_auth_tab() -> Signal<AuthTab> {
    use_context_provider(|| Signal::new(AuthTab::SignIn))
}

pub fn set_auth_tab(tab: AuthTab) {
    use_context::<Signal<AuthTab>>().set(tab);
}
