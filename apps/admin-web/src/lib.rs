//! Console shell assembled from page crates after the frontend split.
pub mod app;
pub mod retro;
pub use app::RootApp;

use app::current_hash;
use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

// Page roots that implement each panel.
use page_account::{KeysPanel, RewardsPanel, SessionsPanel, SettingsPanel, UsageLogsPanel};
use page_admin::{
    AliasesPage, ChannelsPage, GroupsPage, NetworkPanel, RedemptionsPage, SubscriptionsPage,
    SystemPage, state::EntityStore,
};
use page_overview::{LeaderboardPanel, ModelsPanel, OverviewPanel};
use page_users::UsersPanel;

use client::TokenFuture;
use serde::Deserialize;

/// 401 静默刷新接线 (应用启动时由 main 调用一次):
/// - refresher: 读存储的 refresh token → `POST /api/user/refresh` (后端轮换 access+refresh)
///   → 按原持久化级别写回 → 返回新 access token 供客户端对原请求做一次重试;
/// - on_unauthorized: 无 refresh token / 刷新失败 → 清空全部登录态并跳登录页。
pub fn init_auth() {
    let client = client::ApiClient::shared();
    client.set_refresher(start_token_refresh);
    client.set_on_unauthorized(handle_unauthorized);
}

/// 刷新不可恢复时的统一清理: 4 个登录态存储 key + client 内存 token 全部清空,
/// 并把 hash 切到 #signup 让 RootApp 渲染登录页。
fn handle_unauthorized() {
    ui::remove_storage_item("ferrite_access_token");
    ui::remove_storage_item("ferrite_refresh_token");
    ui::remove_storage_item("ferrite_username");
    ui::remove_storage_item("ferrite_current_user");
    client::ApiClient::shared().set_token(None);
    if let Some(w) = web_sys::window() {
        let _ = w.location().set_hash("#signup");
    }
}

/// 一次 access token 刷新。走 `post_once` (刷新请求自身 401 时不再递归刷新);
/// 后端按 `auth::service::RefreshResult` 轮换两枚 token, 均按原持久化级别写回。
fn start_token_refresh() -> TokenFuture {
    Box::pin(async move {
        let refresh_token = match ui::get_storage_item("ferrite_refresh_token") {
            Some(t) if !t.is_empty() => t,
            _ => return None,
        };
        let persistent = ui::token_is_persistent();
        let body = serde_json::json!({ "refreshToken": refresh_token });
        let client = client::ApiClient::shared().clone();
        match client
            .post_once::<_, RefreshWire>("/api/user/refresh", &body, None)
            .await
        {
            Ok(r) => {
                ui::set_storage_scoped("ferrite_access_token", &r.access_token, persistent);
                ui::set_storage_scoped("ferrite_refresh_token", &r.refresh_token, persistent);
                Some(r.access_token)
            }
            Err(_) => None,
        }
    })
}

/// `POST /api/user/refresh` 响应 wire (后端 `RefreshResult`, camelCase)。
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshWire {
    access_token: String,
    refresh_token: String,
}

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
            class: "fixed left-2 top-1/2 z-40 flex -translate-y-1/2 flex-col items-center gap-2.5 p-1 bg-transparent",
            onwheel: move |e: WheelEvent| wheel_step(e, active, &wheel),
            for s in SECTIONS {
                button {
                    key: "{s.label()}",
                    class: if active == s {
                        "h-9 w-2 rounded-full bg-gradient-to-b from-zinc-400 to-zinc-500 shadow-sm shadow-black/60 ring-1 ring-white/10 transition-all hover:from-zinc-300 hover:to-zinc-400"
                    } else {
                        "h-2 w-2 rounded-full bg-zinc-700/60 ring-1 ring-white/5 transition-all hover:bg-zinc-500"
                    },
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

/// 顶栏用户菜单：点击用户名展开下拉,含「账户资料」与「退出登录」。
#[component]
fn UserMenu(name: String, on_logout: EventHandler<()>) -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        div {
            class: "relative",
            button {
                class: "rounded-full bg-zinc-100 px-3 py-1.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-300 inline-flex items-center justify-center",
                "data-testid": "user-menu-button",
                "aria-haspopup": "menu",
                "aria-expanded": "{open()}",
                onclick: move |_| open.toggle(),
                "{name}"
            }
            if open() {
                div {
                    class: "absolute right-0 mt-2 w-44 overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900/95 py-1 text-left shadow-xl shadow-black/40 backdrop-blur",
                    role: "menu",
                    "aria-label": "用户菜单",
                    a {
                        class: "block px-4 py-2.5 text-sm text-zinc-200 transition-colors hover:bg-zinc-800 hover:text-zinc-100",
                        "data-testid": "menu-account",
                        role: "menuitem",
                        href: "#account",
                        onclick: move |_| open.set(false),
                        "账户资料"
                    }
                    div { class: "my-1 h-px bg-zinc-800" }
                    button {
                        class: "block w-full text-left px-4 py-2.5 text-sm text-red-400 transition-colors hover:bg-zinc-800 hover:text-red-300",
                        "data-testid": "logout",
                        role: "menuitem",
                        onclick: move |_| { open.set(false); on_logout.call(()); },
                        "退出登录"
                    }
                }
            }
        }
    }
}

fn get_initial_route() -> (Section, u8) {
    if let Some(w) = web_sys::window()
        && let Ok(loc) = w.location().hash()
    {
        return match loc.as_str() {
            "#overview" => (Section::Dashboard, 0),
            "#models" => (Section::Dashboard, 1),
            "#leaderboard" => (Section::Dashboard, 2),
            "#account" => (Section::Account, 0),
            "#usage" => (Section::Account, 1),
            "#rewards" => (Section::Account, 2),
            "#sessions" => (Section::Account, 3),
            "#settings" => (Section::Account, 4),
            "#manage" | "#network" => (Section::Manage, 0),
            "#users" => (Section::Manage, 1),
            "#groups" => (Section::Manage, 2),
            "#aliases" => (Section::Manage, 3),
            "#channels" => (Section::Manage, 4),
            "#subscriptions" => (Section::Manage, 5),
            "#redemptions" => (Section::Manage, 6),
            "#system" => (Section::Manage, 7),
            _ => (Section::Dashboard, 0),
        };
    }
    (Section::Dashboard, 0)
}

#[component]
pub fn HomePage() -> Element {
    let (init_sec, init_tab) = get_initial_route();
    let mut section = use_signal(move || init_sec);
    let mut dash_tab = use_signal(move || init_tab);
    let mut theme = use_signal(|| Theme::Dark);
    use_context_provider(EntityStore::empty);
    // 启动即从真实后端灌入 分组/渠道/路由单元/模型别名(管理页网络拓扑与别名页吃真数据);
    // 未登录时 401 静默保持空,登录成功后 HomePage 重挂载会再次 hydrate。
    use_effect(move || {
        spawn(async move {
            page_admin::state::EntityStore::hydrate(EntityStore::empty()).await;
        });
    });

    // 启动恢复：localStorage 里有 token → 注入 shared client，顶部显示用户名
    let mut logged_user = use_signal(|| {
        ui::get_storage_item("ferrite_username").filter(|_| ui::get_cached_token().is_some())
    });
    use_hook(move || {
        if let Some(token) = ui::get_cached_token() {
            client::ApiClient::shared().clone().set_token(Some(token));
        }
    });
    let is_light = theme() == Theme::Light;

    // 用户菜单下拉状态 + 退出登录。do_logout 仅捕获 Copy 的 Signal,本身可 Copy,
    // 可在桌面/移动两个 header 分支复用。
    let mut dropdown_open = use_signal(|| false);
    let mut do_logout = move || {
        ui::remove_storage_item("ferrite_access_token");
        ui::remove_storage_item("ferrite_refresh_token");
        ui::remove_storage_item("ferrite_username");
        ui::remove_storage_item("ferrite_current_user");
        client::ApiClient::shared().set_token(None);
        logged_user.set(None);
        dropdown_open.set(false);
        if let Some(w) = web_sys::window() {
            let _ = w.location().set_hash("#signup");
        }
    };

    // 同步 URL hash → 当前 section/tab。HomePage 切到 #auth/#signup/#login/#retro
    // 时会卸载,其 section/dash_tab signal 随之 drop;监听必须在卸载时移除,否则旧
    // listener 在下次 hashchange 写入已释放的 signal 触发 ValueDroppedError panic
    // (原 .forget() 让监听常驻,正是 apps/admin-web/src/lib.rs:215 panic 的根因)。
    let hash_listener = use_signal(|| {
        let mut section_sig = section;
        let mut dash_tab_sig = dash_tab;
        let cb = Closure::<dyn FnMut()>::new(move || {
            let h = current_hash();
            let is_console = h != "#auth" && h != "#signup" && h != "#login" && h != "#retro";
            if !is_console {
                return;
            }
            let (s, t) = get_initial_route();
            section_sig.set(s);
            dash_tab_sig.set(t);
        });
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("hashchange", cb.as_ref().unchecked_ref());
        }
        cb
    });
    use_drop(move || {
        if let Some(w) = web_sys::window() {
            let _ = w.remove_event_listener_with_callback(
                "hashchange",
                hash_listener.read().as_ref().unchecked_ref(),
            );
        }
    });

    // 各 section 的 tab 列表;dash_tab 跨 section 共享,可能越界
    let labels: Vec<String> = match section() {
        Section::Dashboard => vec!["总览".into(), "模型".into(), "排行榜".into()],
        Section::Account => vec![
            "密钥·资料".into(),
            "用量·日志".into(),
            "邀请·奖励".into(),
            "会话".into(),
            "设置".into(),
        ],
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
                    if let Some(name) = logged_user() {
                        UserMenu { name, on_logout: move |_| do_logout() }
                    } else {
                        a {
                            class: "rounded-full bg-zinc-100 px-3 py-1.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-300 inline-flex items-center justify-center",
                            href: "#signup",
                            "登录"
                        }
                    }
                }
            }
            main { class: "flex min-h-0 min-w-0 flex-1 flex-col p-4 sm:p-6 md:pt-20",
                div { class: "mb-4 flex items-center justify-between lg:hidden",
                    span { class: "text-base font-semibold", "Ferrite · 控制台" }
                    // 登录态入口只在桌面 fixed header (UserMenu), 移动行不再重复展示
                    if logged_user().is_none() {
                        a {
                            class: "rounded-full bg-neutral-100 px-3 py-1 text-sm font-medium text-neutral-900 inline-flex items-center justify-center",
                            href: "#signup",
                            "登录"
                        }
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
                        (Section::Account, 3) => rsx! { SessionsPanel {} },
                        (Section::Account, 4) => rsx! { SettingsPanel {} },
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
