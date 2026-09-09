//! Auth page library: provides state context provider plus public entry.

use crate::form::SubmitState;
use crate::state::{AuthTab, use_auth_tab};
use dioxus::prelude::*;
use web_sys::wasm_bindgen::JsCast;
use web_sys::wasm_bindgen::prelude::*;

pub mod api;
pub mod form;
pub mod state;
pub mod view;

fn current_hash() -> String {
    web_sys::window()
        .and_then(|w| w.location().hash().ok())
        .unwrap_or_default()
}

#[component]
pub fn AuthPageRoot() -> Element {
    let tab = use_auth_tab();
    // SubmitState 在根组件 provide，保证所有 auth 子组件（form/view）可见
    use_context_provider(|| SubmitState {
        error: Signal::new(None),
        busy: Signal::new(false),
    });

    // hash → tab：#signup 进入注册表单，#login/#auth 进入登录表单。
    // 否则默认 SignIn，用户从 #signup 进来却看到登录表单，会以为注册坏了。
    let _listener = use_signal(|| {
        let mut tab_sig = tab;
        let to_tab = |h: &str| {
            if h == "#signup" {
                AuthTab::SignUp
            } else {
                AuthTab::SignIn
            }
        };
        tab_sig.set(to_tab(&current_hash()));
        let cb = Closure::<dyn FnMut()>::new(move || {
            tab_sig.set(to_tab(&current_hash()));
        });
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("hashchange", cb.as_ref().unchecked_ref());
        }
        cb
    });
    use_drop(move || {
        if let Some(w) = web_sys::window() {
            let _ = w.remove_event_listener_with_callback(
                "hashchange",
                _listener.read().as_ref().unchecked_ref(),
            );
        }
    });

    rsx! { view::AuthPage {} }
}
