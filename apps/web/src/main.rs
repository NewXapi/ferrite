//! Web entry. Mounts the original console shell.

use dioxus::prelude::*;
use newapi_web_rs::RootApp;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.out.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        RootApp {}
    }
}
