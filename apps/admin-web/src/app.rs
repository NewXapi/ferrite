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

    // 启动恢复 + 401 静默刷新，挂在整个 app 入口（auth 页之外的任何路由都能恢复）：
    // 1) storage 里有 access token → 注入 shared client，刷新页面不掉登录；
    // 2) 注册 refresher：access 过期(15min)时用 refreshToken 换新并重试原请求。
    use_hook(move || {
        let c = client::ApiClient::shared().clone();
        if let Some(token) = ui::get_cached_token() {
            c.set_token(Some(token));
        }
        c.set_refresher(move || {
            let c = client::ApiClient::shared().clone();
            Box::pin(async move {
                match ui::refresh_access_token().await {
                    Some(t) => {
                        c.set_token(Some(t.clone()));
                        Some(t)
                    }
                    None => {
                        // refresh 也失效：清 storage，让下个 401 跳回登录页
                        ui::clear_cached_session();
                        None
                    }
                }
            })
        });
    });

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
