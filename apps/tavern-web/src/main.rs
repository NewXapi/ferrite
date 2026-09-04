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
        TavernApp {}
    }
}
