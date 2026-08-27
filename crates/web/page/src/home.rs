use dioxus::prelude::*;

use crate::auth::auth_drawer::AuthDrawer;
use crate::model::ModelsPanel;
use crate::network::NetworkPanel;
use crate::overview::OverviewPanel;

/// Grayscale theme id; toggling flips a `light` class on the root wrapper.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Dashboard,
    Keys,
    Usage,
    Logs,
    Channels,
    Network,
}

impl Section {
    fn label(self) -> &'static str {
        match self {
            Section::Dashboard => "总览",
            Section::Keys => "密钥",
            Section::Usage => "用量",
            Section::Logs => "日志",
            Section::Channels => "渠道",
            Section::Network => "拓扑",
            }
    }
}
/// Console shell: floating capsule topbar, left navigation rail, windowed canvas
/// with a tab bar, and a right context rail. Grayscale, layout-first, data mocked.
#[component]
pub fn HomePage() -> Element {
    let mut drawer_open = use_signal(|| false);
    let mut section = use_signal(|| Section::Dashboard);
    // Dashboard top-tabs (总览 / 趋势), from the reference overview design.
    let mut dash_tab = use_signal(|| 0u8);
    let mut theme = use_signal(|| Theme::Dark);
    let is_light = theme() == Theme::Light;

    let open_drawer = move |_| drawer_open.set(true);
    let close_drawer = move |_| drawer_open.set(false);
    let panel_header = if section() == Section::Dashboard {
        rsx! {
            div { class: "flex h-full",
                TabItem { label: "总览", active: dash_tab() == 0, onclick: move |_| dash_tab.set(0) }
                TabItem { label: "趋势", active: dash_tab() == 1, onclick: move |_| dash_tab.set(1) }
            }
        }
    } else {
        rsx! {
            div { class: "flex h-full",
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
                    class: "fixed top-4 left-1/2 z-30 hidden -translate-x-1/2 md:block",
                    div { class: "flex items-center gap-5 rounded-full border border-zinc-800/80 bg-zinc-900/75 px-5 py-2.5 backdrop-blur-xl shadow-lg shadow-black/20",
                        // Logo cluster
                        div { class: "flex items-center gap-1.5",
                            span { class: "text-lg font-semibold tracking-tight text-zinc-100", "New API" }
                            span { class: "hidden sm:inline-flex items-center rounded-full bg-zinc-800 px-2 py-0.5 text-xs font-medium uppercase tracking-wider text-zinc-500", "web-rs" }
                        }
                        // Section quick nav (mirrors blocker status on the canvas)
                        nav { class: "hidden lg:flex items-center gap-4",
                            a { class: "text-sm font-medium text-zinc-400 transition-colors hover:text-zinc-100", href: "#",
                                onclick: move |event| { event.prevent_default(); section.set(Section::Keys); },
                                "密钥"
                            }
                            a { class: "text-sm font-medium text-zinc-400 transition-colors hover:text-zinc-100", href: "#",
                                onclick: move |event| { event.prevent_default(); section.set(Section::Usage); },
                                "用量"
                            }
                            a { class: "text-sm font-medium text-zinc-400 transition-colors hover:text-zinc-100", href: "#",
                                onclick: move |event| { event.prevent_default(); section.set(Section::Channels); },
                                "渠道"
                            }
                        }
                        // Divider
                        span { class: "hidden lg:block h-5 w-px bg-zinc-800" }
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

                // ---- Left navigation rail ----
    aside { class: "hidden w-56 shrink-0 flex-col justify-between border-r border-zinc-800 bg-zinc-900/40 lg:flex",
                    div {
                        // Brand
                        div { class: "border-b border-zinc-800 px-4 py-4",
                            div { class: "flex items-baseline gap-2",
                                span { class: "text-base font-semibold tracking-tight", "New API" }
                                span { class: "text-xs text-zinc-500", "控制台" }
                            }
                        }
                        // Workspace nav
                        nav { class: "flex flex-col gap-1.5 px-3 py-3",
                            for s in [
                                Section::Dashboard,
                                Section::Keys,
                                Section::Usage,
                                Section::Logs,
                                Section::Channels,
                                Section::Network,
                            ] {
                                NavItem {
                                    label: s.label(),
                                    active: section() == s,
                                    onclick: move |_| section.set(s),
                                }
                            }
                        }
                    }
                }

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
                            (Section::Dashboard, _) => rsx! { ModelsPanel {} },
                            (Section::Keys, _) => rsx! { PlaceholderPane { text: "API 密钥：列表 + 详情抽屉（占位）" } },
                            (Section::Usage, _) => rsx! { PlaceholderPane { text: "用量明细（占位）" } },
                            (Section::Logs, _) => rsx! { PlaceholderPane { text: "日志流（占位）" } },
                            (Section::Channels, _) => rsx! { PlaceholderPane { text: "渠道管理（占位）" } },
                            (Section::Network, _) => rsx! { NetworkPanel {} },
                        }
                    }
                }

                // ---- Right context rail ----
                aside { class: "hidden w-64 shrink-0 border-l border-zinc-800 bg-zinc-900/40 p-5 xl:block",
                    div { class: "space-y-5",
                        RailItem { label: "状态", value: "正常运行" }
                        RailItem { label: "负载", value: "中等" }
                        RailItem { label: "区间", value: "近 24 小时" }
                        RailItem { label: "渠道健康", value: "12 / 14" }
                        RailItem { label: "可用模型", value: "47" }

                        div { class: "border-t border-zinc-800 pt-4",
                            p { class: "mb-2 text-xs uppercase tracking-wider text-zinc-600", "资源" }
                            div { class: "flex flex-col gap-1.5 text-sm text-zinc-400",
                                a { class: "transition-colors hover:text-zinc-200", href: "#", "API 文档" }
                                a { class: "transition-colors hover:text-zinc-200", href: "#", "定价说明" }
                                a { class: "transition-colors hover:text-zinc-200", href: "#", "更新公告" }
                            }
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
fn NavItem(label: &'static str, active: bool, onclick: EventHandler<MouseEvent>) -> Element {
    let tone = if active {
        "is-pressed bg-zinc-900 border-zinc-600 text-zinc-100"
    } else {
        "bg-zinc-800 border-transparent text-zinc-400 hover:bg-zinc-700 hover:text-zinc-200"
    };
    rsx! {
        button {
            class: "btn-tactile w-full rounded-full border px-3 py-2 text-left text-sm font-medium {tone}",
            onclick: move |event| onclick.call(event),
            "{label}"
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
fn RailItem(label: &'static str, value: &'static str) -> Element {
    rsx! {
        div {
            p { class: "text-xs text-zinc-600", "{label}" }
            p { class: "mt-0.5 text-sm font-medium text-zinc-300", "{value}" }
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
