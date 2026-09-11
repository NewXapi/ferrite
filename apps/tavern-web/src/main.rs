//! Tavern web entry. Mounts the tavern shell.

use dioxus::prelude::*;
use tavern_web_app::TavernApp;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.out.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        // 全局 toast 出口：组件内 ui::components::toast::toast() 触发。
        ui_components::components::toast::Toaster {}
        TavernApp {}
    }
}
