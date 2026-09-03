//! Tavern shell assembled from tavern-web page crates.
//! Mirrors the admin console shell: floating top nav, edge section pill, panel frame.

use dioxus::prelude::*;
use tavern_page_characters::CharactersPage;
use tavern_page_chat::ChatPage;
use tavern_page_settings::SettingsPage;

/// Top-level tavern sections, in navigation order.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Characters,
    Chat,
    Settings,
}

impl Section {
    pub fn label(self) -> &'static str {
        match self {
            Section::Characters => "角色",
            Section::Chat => "聊天",
            Section::Settings => "设置",
        }
    }
}

pub const SECTIONS: [Section; 3] = [Section::Characters, Section::Chat, Section::Settings];

/// Grayscale theme id; toggling flips a `light` class on the root wrapper.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

/// Miniature edge nav pinned to the left: one dot per section, the focused one
/// stretches into a bright bar. Mouse wheel cycles through sections.
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

/// Framed panel with a mac-style header, mirroring the admin console frame.
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
            div { id: "panel-scroll", class: "min-h-0 flex-1 overflow-y-auto overflow-x-hidden p-4 sm:p-6",
                {children}
            }
        }
    }
}

/// Root tavern shell: top nav + section pill + panel hosting the page crates.
#[component]
pub fn TavernApp() -> Element {
    let mut section = use_signal(|| Section::Characters);
    let mut theme = use_signal(|| Theme::Dark);
    let is_light = theme() == Theme::Light;

    rsx! {
        div {
            class: "flex h-screen overflow-hidden bg-zinc-950 text-zinc-100 transition-all duration-300",
            class: if is_light { "light" } else { "" },
            header {
                class: "fixed top-4 left-1/2 z-30 hidden w-max -translate-x-1/2 md:block",
                div { class: "flex items-center gap-5 rounded-full border border-zinc-800/80 bg-zinc-900/90 px-5 py-2.5 shadow-lg shadow-black/20",
                    div { class: "flex items-center gap-1.5",
                        span { class: "text-lg font-semibold tracking-tight text-zinc-100", "Tavern" }
                        span { class: "hidden sm:inline-flex items-center rounded-full bg-zinc-800 px-2 py-0.5 text-xs font-medium uppercase tracking-wider text-zinc-500", "web-rs" }
                    }
                    TopNavMeter { active: section(), on_select: move |s| section.set(s) }
                    button {
                        class: "rounded-full px-3 py-1.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100",
                        onclick: move |_| theme.set(if is_light { Theme::Dark } else { Theme::Light }),
                        if is_light { "Dark" } else { "Light" }
                    }
                }
            }
            SectionPill { active: section(), on_select: move |s| section.set(s) }
            main { class: "flex min-h-0 min-w-0 flex-1 flex-col p-4 sm:p-6 md:pt-20",
                div { class: "mb-4 flex items-center justify-between lg:hidden",
                    span { class: "text-base font-semibold", "Tavern" }
                }
                ConsolePanel {
                    header: rsx! {
                        span { class: "text-sm font-medium text-zinc-300", "{section().label()}" }
                    },
                    match section() {
                        Section::Characters => rsx! { CharactersPage {} },
                        Section::Chat => rsx! { ChatPage {} },
                        Section::Settings => rsx! { SettingsPage {} },
                    }
                }
            }
        }
    }
}
