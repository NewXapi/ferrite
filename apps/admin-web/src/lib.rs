//! Console shell assembled from page crates after the frontend split.
pub mod app;
pub mod retro;
pub use app::RootApp;

use dioxus::prelude::*;

// Page roots that implement each panel.
use page_account::{KeysPanel, RewardsPanel, UsageLogsPanel};
use page_admin::{
    AliasesPage, ChannelsPage, GroupsPage, NetworkPanel, RedemptionsPage, SubscriptionsPage,
    SystemPage, state::EntityStore,
};
use page_overview::{LeaderboardPanel, ModelsPanel, OverviewPanel};
use page_users::UsersPanel;

/// Top-level console sections, in navigation order.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Dashboard,
    Account,
    Manage,
}

impl Section {
    pub fn label(self) -> &'static str {
        match self {
            Section::Dashboard => "总览",
            Section::Account => "账户",
            Section::Manage => "管理",
        }
    }
}

pub const SECTIONS: [Section; 3] = [Section::Dashboard, Section::Account, Section::Manage];

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

/// 面板头部文字 tab:激活项底部白色下划线(激活态用底部 0.5px 白色横条指示)
#[component]
pub fn TabItem(label: String, active: bool, onclick: EventHandler<MouseEvent>) -> Element {
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

#[component]
pub fn HomePage() -> Element {
    let mut section = use_signal(|| Section::Dashboard);
    let mut dash_tab = use_signal(|| 0u8);
    let mut theme = use_signal(|| Theme::Dark);
    use_context_provider(EntityStore::seed);
    let is_light = theme() == Theme::Light;
    // 各 section 的 tab 列表;dash_tab 跨 section 共享,可能越界
    let labels: Vec<String> = match section() {
        Section::Dashboard => vec!["总览".into(), "模型".into(), "排行榜".into()],
        Section::Account => vec!["密钥·资料".into(), "用量·日志".into(), "邀请·奖励".into()],
        Section::Manage => vec![
            "网络".into(),
            "用户".into(),
            "分组".into(),
            "别名".into(),
            "渠道".into(),
            "订阅".into(),
            "兑换".into(),
            "系统".into(),
        ],
    };
    // 越界的 dash_tab clamp 到当前 section 的末位 tab,保证选中态与内容一致
    let active_tab = (dash_tab() as usize).min(labels.len() - 1) as u8;
    let panel_header = {
        let labels = labels.clone();
        let tab_count = labels.len() as i32;
        rsx! {
            div {
                class: "flex h-full min-w-0 overflow-x-auto whitespace-nowrap",
                onwheel: move |e: WheelEvent| {
                    e.prevent_default();
                    use dioxus::html::geometry::WheelDelta;
                    let dy = match e.delta() {
                        WheelDelta::Pixels(v) => v.y,
                        WheelDelta::Lines(v) => v.y,
                        WheelDelta::Pages(v) => v.y,
                    };
                    let next = (dash_tab() as i32 + if dy > 0.0 { 1 } else { -1 }).rem_euclid(tab_count);
                    dash_tab.set(next as u8);
                },
                for (i, label) in labels.iter().enumerate() {
                    TabItem {
                        key: "{i}",
                        label: label.clone(),
                        active: active_tab as usize == i,
                        onclick: move |_| dash_tab.set(i as u8),
                    }
                }
            }
        }
    };

    rsx! {
        div {
            class: "flex h-screen overflow-hidden bg-zinc-950 text-zinc-100 transition-all duration-300",
            class: if is_light { "light" } else { "" },
            header {
                class: "fixed top-4 left-1/2 z-30 hidden w-max -translate-x-1/2 md:block",
                div { class: "flex items-center gap-5 rounded-full border border-zinc-800/80 bg-zinc-900/90 px-5 py-2.5 shadow-lg shadow-black/20",
                    div { class: "flex items-center gap-1.5",
                        span { class: "text-lg font-semibold tracking-tight text-zinc-100", "Ferrite" }
                        span { class: "hidden sm:inline-flex items-center rounded-full bg-zinc-800 px-2 py-0.5 text-xs font-medium uppercase tracking-wider text-zinc-500", "admin" }
                    }
                    TopNavMeter { active: section(), on_select: move |s| section.set(s) }
                    button {
                        class: "rounded-full px-3 py-1.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100",
                        onclick: move |_| theme.set(if is_light { Theme::Dark } else { Theme::Light }),
                        if is_light { "Dark" } else { "Light" }
                    }
                    a {
                        class: "rounded-full bg-zinc-100 px-3 py-1.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-300 inline-flex items-center justify-center",
                        href: "#signup",
                        "登录"
                    }
                }
            }
            SectionPill { active: section(), on_select: move |s| section.set(s) }
            main { class: "flex min-h-0 min-w-0 flex-1 flex-col p-4 sm:p-6 md:pt-20",
                div { class: "mb-4 flex items-center justify-between lg:hidden",
                    span { class: "text-base font-semibold", "Ferrite · 控制台" }
                    a {
                        class: "rounded-full bg-neutral-100 px-3 py-1 text-sm font-medium text-neutral-900 inline-flex items-center justify-center",
                        href: "#signup",
                        "登录"
                    }
                }
                ConsolePanel {
                    header: panel_header,
                    match (section(), active_tab) {
                        (Section::Dashboard, 0) => rsx! { OverviewPanel {} },
                        (Section::Dashboard, 1) => rsx! { ModelsPanel {} },
                        (Section::Dashboard, 2) => rsx! { LeaderboardPanel {} },
                        (Section::Dashboard, _) => rsx! { OverviewPanel {} },
                        (Section::Account, 0) => rsx! { KeysPanel {} },
                        (Section::Account, 1) => rsx! { UsageLogsPanel {} },
                        (Section::Account, 2) => rsx! { RewardsPanel {} },
                        (Section::Account, _) => rsx! { KeysPanel {} },
                        (Section::Manage, 0) => rsx! { NetworkPanel {} },
                        (Section::Manage, 1) => rsx! { UsersPanel {} },
                        (Section::Manage, 2) => rsx! {
                            GroupsPage {}
                        },
                        (Section::Manage, 3) => rsx! {
                            AliasesPage {}
                        },
                        (Section::Manage, 4) => rsx! {
                            ChannelsPage {}
                        },
                        (Section::Manage, 5) => rsx! { SubscriptionsPage {} },
                        (Section::Manage, 6) => rsx! { RedemptionsPage {} },
                        (Section::Manage, 7) => rsx! { SystemPage {} },
                        (Section::Manage, _) => rsx! { NetworkPanel {} },
                    }
                }
            }
        }
    }
}
