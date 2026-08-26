use dioxus::prelude::*;

use crate::auth::auth_drawer::AuthDrawer;
use crate::home::{ConsolePanel, Theme};

/// Playground page (`#retro`): floating capsule topbar + one centered panel, no
/// side rails. The panel body shows a quadrant divider — two diagonal rays that
/// fade out before touching the edges, a soft glow at the crossing, and the
/// action buttons sitting right on the hub.
#[component]
pub fn RetroPage() -> Element {
    let mut drawer_open = use_signal(|| false);
    let mut theme = use_signal(|| Theme::Dark);
    let is_light = theme() == Theme::Light;

    rsx! {
        if drawer_open() {
            div {
                class: "fixed inset-0 z-40 bg-black/35 transition-opacity duration-300",
                onclick: move |_| drawer_open.set(false),
                "aria-hidden": "true",
            }
        }

        div {
            class: "flex min-h-screen bg-zinc-950 text-zinc-100 transition-colors duration-300",
            class: if is_light { "light" } else { "" },
            "aria-hidden": drawer_open(),

            // Floating capsule header — same shell as the console page
            header {
                class: "fixed top-4 left-1/2 z-30 hidden -translate-x-1/2 md:block",
                div { class: "flex items-center gap-5 rounded-full border border-zinc-800/80 bg-zinc-900/75 px-5 py-2.5 backdrop-blur-xl shadow-lg shadow-black/20",
                    div { class: "flex items-center gap-1.5",
                        span { class: "text-lg font-semibold tracking-tight text-zinc-100", "New API" }
                        span { class: "hidden sm:inline-flex items-center rounded-full bg-zinc-800 px-2 py-0.5 text-xs font-medium uppercase tracking-wider text-zinc-500", "retro" }
                    }
                    a {
                        class: "text-sm font-medium text-zinc-400 transition-colors hover:text-zinc-100",
                        href: "#",
                        "控制台"
                    }
                    span { class: "h-5 w-px bg-zinc-800" }
                    button {
                        class: "rounded-full px-3 py-1.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100",
                        onclick: move |_| theme.set(if is_light { Theme::Dark } else { Theme::Light }),
                        if is_light { "Dark" } else { "Light" }
                    }
                    button {
                        class: "rounded-full bg-zinc-100 px-3 py-1.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-300",
                        onclick: move |_| drawer_open.set(true),
                        "登录"
                    }
                }
            }

            // Single centered panel
            main { class: "flex min-w-0 flex-1 flex-col p-4 sm:p-6 md:pt-20",
                ConsolePanel {
                    header: rsx! {
                        div { class: "flex h-full items-center",
                            span { class: "text-sm font-medium text-zinc-100", "四象面板" }
                            span { class: "ml-2 text-xs text-zinc-600", "quadrant mock" }
                        }
                    },
                    // Quadrant scene: a big X of diagonal rays, a four-pointed star
                    // plate hugging the center (blades out on the diagonals, V-notches
                    // on the axes), buttons placed in the hollow center.
                    div { class: "relative h-full min-h-[420px] overflow-hidden",
                        svg {
                            class: "pointer-events-none absolute inset-0 h-full w-full",
                            view_box: "0 0 100 100",
                            preserve_aspect_ratio: "none",
                            defs {
                                linearGradient { id: "rayA", x1: "0", y1: "0", x2: "1", y2: "1",
                                    stop { offset: "0%", stop_color: "#ffffff", stop_opacity: "0" }
                                    stop { offset: "35%", stop_color: "#ffffff", stop_opacity: "0.25" }
                                    stop { offset: "50%", stop_color: "#ffffff", stop_opacity: "0.65" }
                                    stop { offset: "65%", stop_color: "#ffffff", stop_opacity: "0.25" }
                                    stop { offset: "100%", stop_color: "#ffffff", stop_opacity: "0" }
                                }
                                linearGradient { id: "rayB", x1: "1", y1: "0", x2: "0", y2: "1",
                                    stop { offset: "0%", stop_color: "#ffffff", stop_opacity: "0" }
                                    stop { offset: "35%", stop_color: "#ffffff", stop_opacity: "0.25" }
                                    stop { offset: "50%", stop_color: "#ffffff", stop_opacity: "0.65" }
                                    stop { offset: "65%", stop_color: "#ffffff", stop_opacity: "0.25" }
                                    stop { offset: "100%", stop_color: "#ffffff", stop_opacity: "0" }
                                }
                            }
                            line { x1: "10", y1: "10", x2: "90", y2: "90", stroke: "url(#rayA)", stroke_width: "0.55", stroke_linecap: "round" }
                            line { x1: "90", y1: "10", x2: "10", y2: "90", stroke: "url(#rayB)", stroke_width: "0.55", stroke_linecap: "round" }
                            // Four-pointed star plate: blade tips on the diagonals,
                            // concave V-notches on the axes, hollow center.
                            path {
                                d: "M 40 50 L 32 32 L 50 40 L 68 32 L 60 50 L 68 68 L 50 60 L 32 68 Z",
                                fill: "rgba(255, 255, 255, 0.04)",
                                stroke: "rgba(255, 255, 255, 0.35)",
                                stroke_width: "0.3",
                                stroke_linejoin: "round",
                            }
                        }
                        div { class: "retro-glow pointer-events-none absolute left-1/2 top-1/2 h-80 w-80 -translate-x-1/2 -translate-y-1/2 rounded-full" }
                        div { class: "absolute left-1/2 top-1/2 z-10 -translate-x-1/2 -translate-y-1/2",
                            div { class: "grid grid-cols-2 gap-2",
                                button { class: "btn-tactile rounded-full border border-zinc-700 bg-zinc-800 px-4 py-2 text-sm font-medium text-zinc-200 hover:bg-zinc-700", "新建" }
                                button { class: "btn-tactile rounded-full border border-zinc-700 bg-zinc-800 px-4 py-2 text-sm font-medium text-zinc-200 hover:bg-zinc-700", "同步" }
                                button { class: "btn-tactile rounded-full border border-zinc-700 bg-zinc-800 px-4 py-2 text-sm font-medium text-zinc-200 hover:bg-zinc-700", "测试" }
                                button { class: "btn-tactile rounded-full border border-zinc-700 bg-zinc-800 px-4 py-2 text-sm font-medium text-zinc-200 hover:bg-zinc-700", "停用" }
                            }
                        }
                    }
                }
            }
        }

        AuthDrawer { open: drawer_open(), light: is_light, on_close: move |_| drawer_open.set(false) }
    }
}
