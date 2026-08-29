//! Console shell assembled from page crates after the frontend split.
pub mod app;
pub mod retro;
pub use app::RootApp;

use dioxus::prelude::*;

// Page roots that implement each panel.
use page_overview::OverviewPanel;
use page_models::ModelsPanel;
use page_leaderboard::LeaderboardPanel;
use page_admin::{NetworkPanel, state::EntityStore};

/// Top-level console sections, in navigation order.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Dashboard,
    Keys,
    Usage,
    Logs,
    Manage,
}

impl Section {
    pub fn label(self) -> &'static str {
        match self {
            Section::Dashboard => "总览",
            Section::Keys => "密钥",
            Section::Usage => "用量",
            Section::Logs => "日志",
            Section::Manage => "管理",
        }
    }
}

pub const SECTIONS: [Section; 5] = [
    Section::Dashboard,
    Section::Keys,
    Section::Usage,
    Section::Logs,
    Section::Manage,
];

/// Grayscale theme id; toggling flips a `light` class on the root wrapper.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}
/// Miniature top-nav pinned to the left edge: one dot per section, the focused
/// one stretches into a bright bar. Mouse wheel cycles through sections.
#[component]
pub fn SectionPill(active: Section, on_select: EventHandler<Section>) -> Element {
    let wheel = on_select;
    rsx! {
        div {
            class: "fixed left-2 top-1/2 z-40 flex -translate-y-1/2 flex-col items-center gap-2 rounded-full border border-zinc-800/80 bg-zinc-900/70 px-2 py-3 backdrop-blur-xl shadow-lg shadow-black/20",
            onwheel: move |e: WheelEvent| wheel_step(e, active, &wheel),
            for s in SECTIONS {
                button {
                    key: "{s.label()}",
                    class: if active == s { "h-10 w-2.5 rounded-full bg-zinc-100 transition-all" } else { "h-2.5 w-2.5 rounded-full bg-zinc-600 transition-all hover:bg-zinc-400" },
                    "aria-label": "{s.label()}",
                    title: "{s.label()}",
                    onclick: move |_| on_select.call(s),
                }
            }
        }
    }
}

fn step(active: Section, dir: i32) -> Section {
    let idx = SECTIONS.iter().position(|s| *s == active).unwrap_or(0);
    SECTIONS[(idx as i32 + dir).rem_euclid(SECTIONS.len() as i32) as usize]
}

fn wheel_step(e: WheelEvent, active: Section, on_select: &EventHandler<Section>) {
    e.prevent_default();
    use dioxus::html::geometry::WheelDelta;
    let dy = match e.delta() {
        WheelDelta::Pixels(v) => v.y,
        WheelDelta::Lines(v) => v.y,
        WheelDelta::Pages(v) => v.y,
    };
    on_select.call(step(active, if dy > 0.0 { 1 } else { -1 }));
}

/// Top-bar segmented pill: one capsule split into slots.
#[component]
pub fn TopNavMeter(active: Section, on_select: EventHandler<Section>) -> Element {
    let len = SECTIONS.len();
    let wheel = on_select;
    rsx! {
        nav {
            class: "flex h-8 items-center gap-0.5 px-1.5",
            onwheel: move |e: WheelEvent| wheel_step(e, active, &wheel),
            for i in 0..len {
                button {
                    key: "{SECTIONS[i].label()}",
                    class: if active == SECTIONS[i] {
                        if i == 0 { "flex h-6 items-center rounded-l-full bg-zinc-100 px-2 text-xs font-semibold text-zinc-900 transition-all" }
                        else if i == len - 1 { "flex h-6 items-center rounded-r-full bg-zinc-100 px-2 text-xs font-semibold text-zinc-900 transition-all" }
                        else { "flex h-6 items-center bg-zinc-100 px-2 text-xs font-semibold text-zinc-900 transition-all" }
                    } else {
                        if i == 0 { "flex h-6 items-center rounded-l-full px-2 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-800 hover:text-zinc-100" }
                        else if i == len - 1 { "flex h-6 items-center rounded-r-full px-2 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-800 hover:text-zinc-100" }
                        else { "flex h-6 items-center px-2 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-800 hover:text-zinc-100" }
                    },
                    onclick: move |_| on_select.call(SECTIONS[i]),
                    "{SECTIONS[i].label()}"
                }
            }
        }
    }
}

#[component]
pub fn TabItem(label: &'static str, active: bool, onclick: EventHandler<MouseEvent>) -> Element {
    let tone = if active {
        "text-zinc-100"
    } else {
        "text-zinc-500 hover:text-zinc-300"
    };
    rsx! {
        button {
            class: "relative flex h-full items-center px-2 text-sm font-medium transition-colors {tone}",
            onclick: move |event| onclick.call(event),
            "{label}"
            if active {
                span { class: "pointer-events-none absolute inset-x-2 bottom-0 h-0.5 bg-zinc-100" }
            }
        }
    }
}

#[component]
pub fn PlaceholderPane(text: &'static str) -> Element {
    rsx! {
        div { class: "flex h-full items-center justify-center p-10",
            div { class: "rounded-xl border border-dashed border-zinc-700 px-6 py-4 text-sm text-zinc-500",
                "{text}"
            }
        }
    }
}

#[component]
pub fn ConsolePanel(header: Element, children: Element) -> Element {
    rsx! {
        section { class: "flex min-h-0 flex-1 flex-col overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-900/60",
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
            div { class: "min-h-0 flex-1 overflow-y-auto p-4 sm:p-6",
                {children}
            }
        }
    }
}

#[component]
pub fn HomePage() -> Element {
    let mut drawer_open = use_signal(|| false);
    let mut section = use_signal(|| Section::Dashboard);
    let mut dash_tab = use_signal(|| 0u8);
    let mut theme = use_signal(|| Theme::Dark);
    use_context_provider(EntityStore::seed);
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
            header {
                class: "fixed top-4 left-1/2 z-30 hidden w-max -translate-x-1/2 md:block",
                div { class: "flex items-center gap-5 rounded-full border border-zinc-800/80 bg-zinc-900/90 px-5 py-2.5 shadow-lg shadow-black/20",
                    div { class: "flex items-center gap-1.5",
                        span { class: "text-lg font-semibold tracking-tight text-zinc-100", "New API" }
                        span { class: "hidden sm:inline-flex items-center rounded-full bg-zinc-800 px-2 py-0.5 text-xs font-medium uppercase tracking-wider text-zinc-500", "web-rs" }
                    }
                    TopNavMeter { active: section(), on_select: move |s| section.set(s) }
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
            main { class: "flex min-w-0 flex-1 flex-col p-4 sm:p-6 md:pt-20",
                div { class: "mb-4 flex items-center justify-between lg:hidden",
                    span { class: "text-base font-semibold", "New API · 控制台" }
                    button {
                        class: "rounded-full bg-neutral-100 px-3 py-1 text-sm font-medium text-neutral-900",
                        onclick: open_drawer,
                        "登录"
                    }
                }
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
                        (Section::Manage, _) => rsx! { NetworkPanel {} },
                    }
                }
            }
        }
        AuthDrawer { open: drawer_open(), light: is_light, on_close: close_drawer }
    }
}

/// Auth page content is owned by the auth page crate; the shell only hosts it.
#[component]
pub fn AuthPageContent() -> Element {
    rsx! { page_auth::AuthPageRoot {} }
}
/// Right-side auth drawer.
#[component]
pub fn AuthDrawer(open: bool, light: bool, on_close: EventHandler<MouseEvent>) -> Element {
    let drawer_class = if open { "translate-x-0" } else { "translate-x-full pointer-events-none" };
    rsx! {
        aside {
            class: "fixed top-0 right-0 z-50 h-full w-full max-w-sm border-l border-zinc-800 bg-zinc-950 shadow-lg transition-transform duration-300 ease-out {drawer_class}",
            class: if light { "light" } else { "" },
            role: "dialog",
            "aria-modal": "true",
            "aria-labelledby": "auth-drawer-title",
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
            div { class: "flex h-full flex-col overflow-y-auto p-6 sm:p-8",
                AuthPageContent {}
            }
        }
    }
}
