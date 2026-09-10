//! Web entry. Mounts the original console shell.

use admin_web::RootApp;
use dioxus::prelude::*;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.out.css");
const DXC_THEME_CSS: Asset = asset!("/assets/dx-components-theme.css");

fn main() {
    // 401 静默刷新 + 过期清登录态的接线必须在首帧前注册。
    admin_web::init_auth();
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        document::Stylesheet { href: DXC_THEME_CSS }
        RootApp {}
    }
}
