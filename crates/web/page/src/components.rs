//! Reusable UI pieces.

use dioxus::html::geometry::WheelDelta;
use dioxus::prelude::*;

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
    let dy = match e.delta() {
        WheelDelta::Pixels(v) => v.y,
        WheelDelta::Lines(v) => v.y,
        WheelDelta::Pages(v) => v.y,
    };
    on_select.call(step(active, if dy > 0.0 { 1 } else { -1 }));
}

/// Top-bar segmented pill: one capsule split into slots — rounded outer ends,
/// flat inner joints. The focused section is the only bright slot.
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
