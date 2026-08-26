use dioxus::prelude::*;

use crate::auth::auth_page::AuthPageContent;

/// Right-side auth drawer. Opens from the right on desktop, nearly full-width on mobile.
/// `open` controls visibility; `on_close` is called when user clicks close or the backdrop.
/// The drawer animator is CSS-driven via `translate-x` so no animation library is needed.
#[component]
pub fn AuthDrawer(open: bool, light: bool, on_close: EventHandler<MouseEvent>) -> Element {
    // Drawer is always rendered so CSS transitions can animate it in/out.
    let drawer_class = if open {
        "translate-x-0"
    } else {
        "translate-x-full pointer-events-none"
    };

    rsx! {
        // Drawer panel — backdrop is managed by the parent (home.rs).
        aside {
            class: "fixed top-0 right-0 z-50 h-full w-full max-w-sm border-l border-zinc-800 bg-zinc-950 shadow-lg transition-transform duration-300 ease-out {drawer_class}",
            class: if light { "light" } else { "" },
            role: "dialog",
            "aria-modal": "true",
            "aria-labelledby": "auth-drawer-title",

            // Close button
            button {
                class: "absolute top-4 right-4 z-10 rounded-lg p-1.5 text-zinc-500 transition-colors hover:text-zinc-200 hover:bg-zinc-800",
                onclick: move |event| on_close.call(event),
                "aria-label": "Close authentication drawer",
                svg {
                    class: "h-5 w-5",
                    fill: "none",
                    stroke: "currentColor",
                    view_box: "0 0 24 24",
                    stroke_width: "2",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18L18 6M6 6l12 12" }
                }
            }

            // Drawer content
            div { class: "flex h-full flex-col overflow-y-auto p-6 sm:p-8",
                AuthPageContent {}
            }
        }
    }
}
