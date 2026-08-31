//! Root application switcher.
use super::HomePage;
use crate::retro::RetroPage;
use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

/// Root application: `#retro` selects the topology branch's experimental page;
/// every other hash renders the regular console.
#[component]
pub fn RootApp() -> Element {
    let mut retro = use_signal(|| current_hash() == "#retro");
    use_hook(move || {
        let cb = Closure::<dyn FnMut()>::new(move || retro.set(current_hash() == "#retro"));
        window().set_onhashchange(Some(cb.as_ref().unchecked_ref()));
        cb.forget();
    });
    rsx! {
        if retro() { RetroPage {} } else { HomePage {} }
    }
}

fn window() -> web_sys::Window {
    web_sys::window().expect("browser window")
}

fn current_hash() -> String {
    window().location().hash().unwrap_or_default()
}
