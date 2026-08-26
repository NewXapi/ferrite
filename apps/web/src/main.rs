use dioxus::prelude::*;
use newapi_page::app::RootApp;

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
