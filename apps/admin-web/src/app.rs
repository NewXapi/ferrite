//! Root application switcher.
use super::HomePage;
use crate::retro::RetroPage;
use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

/// Root application switcher: `#retro` → topology branch page,
/// `#auth` / `#signup` / `#login` → full standalone auth page,
/// otherwise the regular console.
#[component]
pub fn RootApp() -> Element {
    let mut retro = use_signal(|| current_hash() == "#retro");
    let mut is_auth = use_signal(|| is_auth_hash(&current_hash()));

    use_hook(move || {
        let cb = Closure::<dyn FnMut()>::new(move || {
            let h = current_hash();
            retro.set(h == "#retro");
            is_auth.set(is_auth_hash(&h));
        });
        let _ =
            window().add_event_listener_with_callback("hashchange", cb.as_ref().unchecked_ref());
        cb.forget();
    });

    rsx! {
        if is_auth() {
            page_auth::AuthPageRoot {}
        } else if retro() {
            RetroPage {}
        } else {
            HomePage {}
        }
    }
}

pub(crate) fn is_auth_hash(h: &str) -> bool {
    h == "#auth" || h == "#signup" || h == "#login"
}

fn window() -> web_sys::Window {
    web_sys::window().expect("browser window")
}

pub(crate) fn current_hash() -> String {
    window().location().hash().unwrap_or_default()
}
