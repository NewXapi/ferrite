//! Web entry. Mounts the original console shell.

use admin_web::RootApp;
use dioxus::prelude::*;

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
