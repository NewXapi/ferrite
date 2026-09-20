//! Console shell assembled from page crates after the frontend split.
pub mod app;
pub mod retro;
pub use app::RootApp;

use app::{current_hash, is_auth_hash};
use dioxus::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

// Page roots that implement each panel.
use page_account::{KeysPanel, RewardsPanel, SessionsPanel, SettingsPanel, UsageLogsPanel};
use page_admin::{
    AliasesPage, ChannelsPage, CurrencyPage, GatewayHealthPanel, GroupsPage, NetworkPanel,
    RedemptionsPage, SubscriptionsPage, SystemPage, state::EntityStore,
};
use page_overview::{LeaderboardPanel, ModelsPanel, OverviewPanel};
use page_users::UsersPanel;

use client::TokenFuture;
use serde::Deserialize;
use ui::components::layout::{AppShell, SectionRail, StatusBar, TopNavBar};

/// 401 静默刷新接线 (应用启动时由 main 调用一次):
/// - refresher: 读存储的 refresh token → `POST /api/user/refresh` (后端轮换 access+refresh)
///   → 按原持久化级别写回 → 返回新 access token 供客户端对原请求做一次重试;
/// - on_unauthorized: 无 refresh token / 刷新失败 → 清空全部登录态并跳登录页。
pub fn init_auth() {
    let client = client::ApiClient::shared();
    client.set_refresher(start_token_refresh);
    client.set_on_unauthorized(handle_unauthorized);
}

/// debug-auto-login 触发判据（纯函数，不碰 window，供 `tests/debug_login.rs` 单测）：
/// feature 开 + 当前无 token + 当前 hash 不是登录/注册页（`#login`/`#signup`/`#auth`，
/// 对齐 `app.rs` 的 `is_auth_hash`；空 hash 视作普通页）。
/// 调用方以 `cfg!(feature = "debug-auto-login")` 传入 `feature_on` —— feature 关时
/// 恒 false，行为与现状完全一致；主动「退出登录」(do_logout) 不走本判据，
/// 不会被自动重登顶掉。
#[must_use]
pub fn should_debug_auto_login(feature_on: bool, has_token: bool, hash: &str) -> bool {
    feature_on && !has_token && !is_auth_hash(hash)
}

/// dev 种子账号（同 `db/dev/generate_seed.py` 的 admin_dev，role=root）。
/// 仅 debug-auto-login feature 引用，feature 默认关 → 凭据不进生产产物；
/// 生产凭据仍由真实登录流程注入，此常量不是生产密钥。
#[cfg(feature = "debug-auto-login")]
const DEBUG_AUTO_LOGIN_USERNAME: &str = "admin_dev";
#[cfg(feature = "debug-auto-login")]
const DEBUG_AUTO_LOGIN_PASSWORD: &str = "DevPassw0rd!12345";

/// debug-auto-login 执行体：调 `login_api` 登录 dev 种子账号，成功后按登录页
/// `handle_submit` 的同款语义写登录态 —— access/refresh/username/current_user
/// 全部持久档（localStorage，相当于勾选 Remember me），并注入 shared client。
/// 成功/失败都打显式 console 日志；失败不修改任何既有状态（回落现状）。
#[cfg(feature = "debug-auto-login")]
async fn debug_auto_login() -> bool {
    let client = client::ApiClient::shared().clone();
    let result = page_auth::api::login_api(
        &client,
        &page_auth::api::contract_auth::LoginRequest {
            username: DEBUG_AUTO_LOGIN_USERNAME.to_string(),
            password: DEBUG_AUTO_LOGIN_PASSWORD.to_string(),
        },
    )
    .await;
    match result {
        Ok(resp) => {
            client.set_token(Some(resp.access_token.clone()));
            ui::set_storage_scoped("ferrite_access_token", &resp.access_token, true);
            ui::set_storage_scoped("ferrite_refresh_token", &resp.refresh_token, true);
            ui::set_storage_scoped("ferrite_username", DEBUG_AUTO_LOGIN_USERNAME, true);
            // ferrite_current_user 存序列化 UserDto，供顶栏/账户页读取（同 keys.rs 写入口径）
            if let Ok(s) = serde_json::to_string(&resp.user) {
                ui::set_storage_scoped("ferrite_current_user", &s, true);
            }
            web_sys::console::warn_1(
                &format!(
                    "[debug-auto-login] 已自动登录 {DEBUG_AUTO_LOGIN_USERNAME} (dev 种子, 仅 debug 构建生效)"
                )
                .into(),
            );
            true
        }
        Err(e) => {
            web_sys::console::warn_1(
                &format!("[debug-auto-login] 自动登录失败，不做任何事（回落现状）: {e}").into(),
            );
            false
        }
    }
}

/// 触发点 a —— RootApp 启动（app.rs 的启动恢复 use_hook 之后调用一次）：
/// 判据命中 → 异步登录，成功后整页 reload（复用既有「启动恢复」路径，选择理由见
/// app.rs 注释）；失败只留日志，不打断当前渲染（回落未登录现状）。
#[cfg(feature = "debug-auto-login")]
pub(crate) fn debug_auto_login_on_boot() {
    let has_token = ui::get_cached_token().is_some();
    if !should_debug_auto_login(true, has_token, &current_hash()) {
        return;
    }
    spawn(async move {
        if debug_auto_login().await
            && let Some(w) = web_sys::window()
        {
            let _ = w.location().reload();
        }
    });
}

/// 触发点 b —— 401 清会话后的自动重登。同页只发起一次（并发 401 风暴下多个请求
/// 各自触发 handle_unauthorized，不能每个都 spawn 登录+reload）；成功 reload 停留
/// 当前页，失败纯静默回落（不 set_hash，跳 #signup 统一由 handle_unauthorized 原路径
/// 或下一次 401 兜底负责——本函数失败路径的 set_hash 会与外层清理尾部/后续 handler
/// 的 set_hash 并发竞态）。用 `spawn_local` 而非 dioxus `spawn`：
/// 本函数在异步请求 poll 途中被同步回调，不保证处于 reactive scope。
#[cfg(feature = "debug-auto-login")]
fn debug_auto_login_after_unauthorized() {
    use std::cell::Cell;
    thread_local! {
        /// 本次页面加载内是否已发起过 debug 自动登录（防并发 401 重复 spawn+reload）。
        static RELOGIN_SPAWNED: Cell<bool> = const { Cell::new(false) };
    }
    if RELOGIN_SPAWNED.with(Cell::get) {
        return;
    }
    RELOGIN_SPAWNED.with(|c| c.set(true));
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async {
        let ok = debug_auto_login().await;
        if !ok {
            // 失败不 set_hash（纯静默回落）——与 handle_unauthorized 尾部/后续 401 的
            // set_hash("#signup") 并发竞态，跳登录页统一由那条原路径兜底负责。
            return;
        }
        if let Some(w) = web_sys::window() {
            let _ = w.location().reload();
        }
    });
    #[cfg(not(target_arch = "wasm32"))]
    {
        // 原生目标无浏览器环境，仅保证 feature 下可编译；debug 前端只在 wasm32 运行。
    }
}

/// 刷新不可恢复时的统一清理: 4 个登录态存储 key + client 内存 token 全部清空,
/// 并把 hash 切到 #signup 让 RootApp 渲染登录页。
/// debug-auto-login 开启时: 清空后先用 dev 种子账号尝试自动重登（成功 reload
/// 停留当前页）；失败纯静默回落，跳 #signup 由下方尾部/后续 401 兜底。
fn handle_unauthorized() {
    ui::remove_storage_item("ferrite_access_token");
    ui::remove_storage_item("ferrite_refresh_token");
    ui::remove_storage_item("ferrite_username");
    ui::remove_storage_item("ferrite_current_user");
    client::ApiClient::shared().set_token(None);
    // 登录态刚被清空，has_token 恒 false；hash 取当前值（登录页本身不会走到这里，
    // 判据自会拦下 #login/#signup/#auth）。本块仅 feature 开时编译，无 feature 零变化。
    #[cfg(feature = "debug-auto-login")]
    {
        if should_debug_auto_login(true, false, &current_hash()) {
            debug_auto_login_after_unauthorized();
            return;
        }
    }
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
            "#gw-health" => (Section::Manage, 8),
            "#currency" => (Section::Manage, 9),
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
            "网关健康".into(),
            "货币".into(),
        ],
    };
    // 越界的 dash_tab clamp 到当前 section 的末位 tab,保证选中态与内容一致
    let active_tab = (dash_tab() as usize).min(labels.len() - 1) as u8;
    // rail 激活项 = 当前 section 在 SECTIONS 里的下标
    let section_idx = SECTIONS.iter().position(|s| *s == section()).unwrap_or(0);
    rsx! {
        div {
            class: "h-svh overflow-hidden bg-zinc-950 text-zinc-100 transition-all duration-300",
            class: if is_light { "light" } else { "" },
            AppShell {
                rail: rsx! {
                    SectionRail {
                        active_index: section_idx,
                        on_select: move |idx| section.set(SECTIONS[idx]),
                    }
                },
                top_nav: rsx! {
                    TopNavBar {
                        tabs: labels.clone(),
                        active: active_tab as usize,
                        on_select: move |i| dash_tab.set(i as u8),
                    }
                },
                status_bar: rsx! {
                    StatusBar {
                        user_name: logged_user(),
                        is_light: is_light,
                        on_toggle_theme: move |_| theme.set(if is_light { Theme::Dark } else { Theme::Light }),
                        on_logout: move |_| do_logout(),
                    }
                },
                ConsolePanel {
                    header: rsx! { span { class: "text-xs font-medium text-zinc-500", "Ferrite · admin" } },
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
                        (Section::Manage, 9) => rsx! { CurrencyPage {} },
                        (Section::Manage, 8) => rsx! {
                            GatewayHealthPanel {}
                        },
                        (Section::Manage, _) => rsx! { NetworkPanel {} },
                    }
                }
            }
        }
    }
}
