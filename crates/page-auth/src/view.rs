//! Auth page rendering and component composition.
//! Top-anchored layout: logo top-left; tab + title embedded in card.

use crate::form::{SignInForm, SignUpForm};
use crate::state::{AuthTab, auth_tab};
use dioxus::prelude::*;

#[component]
pub fn AuthPage() -> Element {
    let active = auth_tab();
    let is_sign_in = active == AuthTab::SignIn;

    let indicator_transform = if is_sign_in {
        "translate-x-0"
    } else {
        "translate-x-full"
    };
    let sign_in_class = if is_sign_in {
        "text-zinc-900"
    } else {
        "text-zinc-400 hover:text-zinc-200"
    };
    let register_class = if is_sign_in {
        "text-zinc-400 hover:text-zinc-200"
    } else {
        "text-zinc-900"
    };
    let title_text = if is_sign_in {
        "Welcome back"
    } else {
        "Create your account"
    };
    let subtitle_text = if is_sign_in {
        "Sign in to continue to Ferrite"
    } else {
        "Get started with Ferrite in seconds"
    };

    rsx! {
        div {
            class: "relative min-h-screen overflow-hidden bg-zinc-950 text-zinc-100",

            // Background: dim grid (hairline)
            svg {
                class: "absolute inset-0 h-full w-full",
                xmlns: "http://www.w3.org/2000/svg",
                defs {
                    pattern {
                        id: "grid",
                        width: "80",
                        height: "80",
                        pattern_units: "userSpaceOnUse",
                        path {
                            d: "M 80 0 L 0 0 0 80",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "0.5",
                            class: "text-zinc-800/50",
                        }
                    }
                }
                rect {
                    width: "100%",
                    height: "100%",
                    fill: "url(#grid)",
                }
            }

            // Subtle top-left glow
            div {
                class: "absolute -top-32 -left-32 h-96 w-96 rounded-full bg-gradient-to-br from-zinc-800/15 via-zinc-900/25 to-transparent blur-3xl pointer-events-none",
            }

            // Header: logo left only
            header {
                class: "sticky top-0 z-20 flex items-center px-8 py-5 sm:px-12",
                div {
                    class: "flex items-center gap-3",
                    div {
                        class: "flex h-9 w-9 items-center justify-center rounded-lg border border-zinc-800/60 bg-zinc-900/80",
                        svg {
                            class: "h-4 w-4 text-zinc-200",
                            xmlns: "http://www.w3.org/2000/svg",
                            view_box: "0 0 24 24",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            path { d: "M12 2L2 7l10 5 10-5-10-5z" }
                        }
                    }
                    span {
                        class: "text-lg font-semibold tracking-tight text-zinc-100",
                        "Ferrite"
                    }
                }
            }

            // Main: top-anchored card with embedded title + tab
            main {
                class: "relative z-10 w-full max-w-md ml-auto mr-auto px-4 sm:px-6 pt-6 sm:pt-10 pb-16",

                // Card: title + tab + form
                div {
                    class: "rounded-xl border border-zinc-800/70 bg-zinc-900/40 backdrop-blur-sm p-5 sm:p-8 transition-all duration-300",

                    // Card header: title left, tab right
                    div {
                        class: "flex items-start justify-between gap-4 mb-8",

                        // Title block
                        div {
                            class: "min-w-0 flex-1",
                            h1 {
                                class: "text-lg sm:text-xl font-semibold tracking-tight text-zinc-50 leading-tight",
                                "{title_text}"
                            }
                            p {
                                class: "mt-1.5 text-xs sm:text-sm text-zinc-500",
                                "{subtitle_text}"
                            }
                        }

                        // Compact segmented tab (right side of card header)
                        div {
                            class: "relative flex shrink-0 rounded-full border border-zinc-800/80 bg-zinc-950/60 p-0.5",
                            role: "tablist",
                            "aria-label": "Auth mode switcher",

                            // Sliding indicator pill
                            div {
                                class: "absolute top-0.5 bottom-0.5 left-0.5 w-[calc(50%-2px)] rounded-full bg-zinc-100 transition-transform duration-300 ease-out {indicator_transform}",
                                "aria-hidden": "true",
                            }

                            button {
                                class: "relative z-10 rounded-full px-3.5 py-1.5 text-xs font-medium transition-colors duration-200 {sign_in_class}",
                                r#type: "button",
                                role: "tab",
                                aria_selected: "{is_sign_in}",
                                onclick: move |_| crate::state::set_auth_tab(AuthTab::SignIn),
                                "Sign in"
                            }
                            button {
                                class: "relative z-10 rounded-full px-3.5 py-1.5 text-xs font-medium transition-colors duration-200 {register_class}",
                                r#type: "button",
                                role: "tab",
                                aria_selected: "{!is_sign_in}",
                                onclick: move |_| crate::state::set_auth_tab(AuthTab::SignUp),
                                "Register"
                            }
                        }
                    }

                    // Form content
                    match active {
                        AuthTab::SignIn => rsx! { SignInForm {} },
                        AuthTab::SignUp => rsx! { SignUpForm {} },
                    }
                }

                // Footer
                p {
                    class: "mt-8 text-center text-xs text-zinc-600",
                    "By continuing, you agree to our "
                    a {
                        class: "text-zinc-500 hover:text-zinc-400 underline underline-offset-2 transition-colors",
                        href: "#",
                        "Terms of Service"
                    }
                }
            }
        }
    }
}
