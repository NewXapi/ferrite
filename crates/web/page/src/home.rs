use dioxus::prelude::*;

use crate::auth::auth_drawer::AuthDrawer;
use crate::components::{Section, SectionPill, TopNavMeter};
use crate::leaderboard::LeaderboardPanel;
use crate::model::ModelsPanel;
use crate::network::NetworkPanel;
use crate::overview::OverviewPanel;

/// Grayscale theme id; toggling flips a `light` class on the root wrapper.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

/// Console shell: floating capsule topbar, one full-width windowed canvas with a
/// tab bar, GNOME-style nav pill on the left edge, and a meter in the capsule.
#[component]
pub fn HomePage() -> Element {
    let mut drawer_open = use_signal(|| false);
    let mut section = use_signal(|| Section::Dashboard);
    // Dashboard top-tabs (总览 / 模型), from the reference overview design.
    let mut dash_tab = use_signal(|| 0u8);
    let mut theme = use_signal(|| Theme::Dark);
    let is_light = theme() == Theme::Light;

    let open_drawer = move |_| drawer_open.set(true);
    let close_drawer = move |_| drawer_open.set(false);
    let panel_header = if section() == Section::Dashboard {
        rsx! {
            div { class: "flex h-full min-w-0 overflow-x-auto whitespace-nowrap",
                TabItem { label: "总览", active: dash_tab() == 0, onclick: move |_| dash_tab.set(0) }
                TabItem { label: "模型", active: dash_tab() == 1, onclick: move |_| dash_tab.set(1) }
                TabItem { label: "排行榜", active: dash_tab() == 2, onclick: move |_| dash_tab.set(2) }
            }
        }
    } else {
        rsx! {
            div { class: "flex h-full min-w-0 overflow-x-auto whitespace-nowrap",
                TabItem { label: section().label(), active: true, onclick: move |_| {} }
            }
        }
    };

    rsx! {
        if drawer_open() {
            div {
                class: "fixed inset-0 z-40 bg-black/35 transition-opacity duration-300",
                onclick: close_drawer,
                "aria-hidden": "true",
            }
        }

        div {
            class: "flex min-h-screen bg-zinc-950 text-zinc-100 transition-all duration-300",
            class: if drawer_open() { "brightness-75" } else { "" },
            class: if is_light { "light" } else { "" },
            "aria-hidden": drawer_open(),

            // Floating capsule header (overrides the in-titlebar row on desktop)
            header {
                class: "fixed top-4 left-1/2 z-30 hidden w-max -translate-x-1/2 md:block",
                div { class: "flex items-center gap-5 rounded-full border border-zinc-800/80 bg-zinc-900/90 px-5 py-2.5 shadow-lg shadow-black/20",
                    // Logo cluster
                    div { class: "flex items-center gap-1.5",
                        span { class: "text-lg font-semibold tracking-tight text-zinc-100", "New API" }
                        span { class: "hidden sm:inline-flex items-center rounded-full bg-zinc-800 px-2 py-0.5 text-xs font-medium uppercase tracking-wider text-zinc-500", "web-rs" }
                    }
                    TopNavMeter { active: section(), on_select: move |s| section.set(s) }
                    // Theme + sign-in
                    button {
                        class: "rounded-full px-3 py-1.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100",
                        onclick: move |_| theme.set(if is_light { Theme::Dark } else { Theme::Light }),
                        if is_light { "Dark" } else { "Light" }
                    }
                    button {
                        class: "rounded-full bg-zinc-100 px-3 py-1.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-300",
                        onclick: open_drawer,
                        "登录"
                    }
                }
            }

            SectionPill { active: section(), on_select: move |s| section.set(s) }

            // ---- Center: windowed canvas ----
                main { class: "flex min-w-0 flex-1 flex-col p-4 sm:p-6 md:pt-20",
                // Mobile brand row
                div { class: "mb-4 flex items-center justify-between lg:hidden",
                    span { class: "text-base font-semibold", "New API · 控制台" }
                    button {
                        class: "rounded-full bg-neutral-100 px-3 py-1 text-sm font-medium text-neutral-900",
                        onclick: open_drawer,
                        "登录"
                    }
                }

                // Frame — one shared panel keeps every page's chrome identical
                ConsolePanel {
                    header: panel_header,
                    match (section(), dash_tab()) {
                        (Section::Dashboard, 0) => rsx! { OverviewPanel {} },
                        (Section::Dashboard, 1) => rsx! { ModelsPanel {} },
                        (Section::Dashboard, 2) => rsx! { LeaderboardPanel {} },
                        (Section::Dashboard, _) => rsx! { OverviewPanel {} },
                        (Section::Keys, _) => rsx! { PlaceholderPane { text: "API 密钥：列表 + 详情抽屉（占位）" } },
                        (Section::Usage, _) => rsx! { PlaceholderPane { text: "用量明细（占位）" } },
                        (Section::Logs, _) => rsx! { PlaceholderPane { text: "日志流（占位）" } },
                        (Section::Channels, _) => rsx! { PlaceholderPane { text: "渠道管理（占位）" } },
                        (Section::Network, _) => rsx! { NetworkPanel {} },
                    }
                }
            }
        }

        AuthDrawer { open: drawer_open(), light: is_light, on_close: close_drawer }
    }
}

/// One console window panel: rounded frame + fixed-height title bar (header left,
/// window dots right) + padded scrollable body. Every section page renders through
/// this so the chrome (height, padding, border) stays identical across tabs.
#[component]
pub(crate) fn ConsolePanel(header: Element, children: Element) -> Element {
    rsx! {
        section { class: "flex min-h-0 flex-1 flex-col overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-900/60",
            // Title bar: fixed height so tabs and plain labels render at the same size
            div { class: "flex h-9 shrink-0 items-center justify-between border-b border-zinc-800 px-4",
                {header}
                div { class: "flex items-center gap-1.5",
                    for _ in 0..3 {
                        button {
                            class: "h-3.5 w-3.5 rounded-full border border-zinc-700 bg-zinc-800 transition-colors hover:bg-zinc-700",
                            "aria-label": "window control",
                        }
                    }
                }
            }
            // Canvas body
            div { class: "min-h-0 flex-1 overflow-y-auto p-4 sm:p-6",
                {children}
            }
        }
    }
}

#[component]
fn TabItem(label: &'static str, active: bool, onclick: EventHandler<MouseEvent>) -> Element {
    let tone = if active {
        "text-zinc-100"
    } else {
        "text-zinc-500 hover:text-zinc-300"
    };
    rsx! {
        button {
            class: "relative flex h-full shrink-0 items-center px-2 text-sm font-medium transition-colors {tone}",
            onclick: move |event| onclick.call(event),
            "{label}"
            if active {
                span { class: "pointer-events-none absolute inset-x-2 bottom-0 h-0.5 bg-zinc-100" }
            }
        }
    }
}

/// Placeholder pane for not-yet-built sections.
#[component]
fn PlaceholderPane(text: &'static str) -> Element {
    rsx! {
        div { class: "flex h-full items-center justify-center p-10",
            div { class: "rounded-xl border border-dashed border-zinc-700 px-6 py-4 text-sm text-zinc-500",
                "{text}"
            }
        }
    }
}
