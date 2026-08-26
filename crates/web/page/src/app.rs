use dioxus::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::home::HomePage;
use crate::retro::RetroPage;

/// Route by URL hash: `#retro` renders the experimental retro page, anything
/// else renders the regular console. Keeps the scratch page off the main one.
#[component]
pub fn RootApp() -> Element {
    let mut retro = use_signal(|| current_hash() == "#retro");
    use_hook(move || {
        let cb = Closure::<dyn FnMut()>::new(move || retro.set(current_hash() == "#retro"));
        window().set_onhashchange(Some(cb.as_ref().unchecked_ref()));
        cb.forget(); // ponytail: leaks one closure for the app lifetime — fine
    });
    rsx! {
        if retro() {
            RetroPage {}
        } else {
            HomePage {}
        }
    }
}

fn window() -> web_sys::Window {
    web_sys::window().expect("browser window")
}

fn current_hash() -> String {
    window().location().hash().unwrap_or_default()
}
