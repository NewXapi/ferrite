//! 复刻 admin-web 的控制台壳 + 用户页。
//!
//! Hydration 版：首屏 SSR 吐 HTML，之后筛选/启停全在客户端信号上，
//! 不刷页面。样式内联在 shell 里，无构建步骤。

use std::sync::Arc;

use leptos::prelude::*;
use leptos::server_fn::ServerFn;

use crate::users::{cny, Filter, ListUsers, PageState, ToggleUser, User};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="zh">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <title>"Ferrite · admin (Leptos)"</title>
                <style>{STYLE}</style>
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <div class="shell">
            <Rail />
            <div class="main">
                <TopBar />
                <section class="panel">
                    <div class="panel-bar">"Ferrite · admin"</div>
                    <div class="panel-body">
                        <UsersPage />
                    </div>
                </section>
                <div class="status">"admin_dev"</div>
            </div>
        </div>
    }
}

#[component]
fn Rail() -> impl IntoView {
    view! {
        <nav class="rail">
            <span class="dot" title="总览"></span>
            <span class="dot" title="账户"></span>
            <span class="dot active" title="管理"></span>
        </nav>
    }
}

#[component]
fn TopBar() -> impl IntoView {
    let tabs = [
        "网络", "用户", "分组", "别名", "渠道",
        "订阅", "兑换", "系统", "网关健康", "货币",
    ];
    view! {
        <nav class="tabs">
            {tabs
                .into_iter()
                .map(|tab| {
                    let active = tab == "用户";
                    view! {
                        <span class=if active { "tab active" } else { "tab" }>{tab}</span>
                    }
                })
                .collect_view()}
        </nav>
    }
}

/// 用户页。users 信号 hydration 后在客户端；筛选和启停都改本地信号。
#[component]
fn UsersPage() -> impl IntoView {
    // SSR 直读进程内状态；CSR 才走 HTTP 打 server function。
    let users = Resource::new(
        || (),
        |_| async {
            #[cfg(feature = "ssr")]
            {
                let state = use_context::<std::sync::Arc<tokio::sync::Mutex<PageState>>>()
                    .unwrap_or_else(|| std::sync::Arc::new(tokio::sync::Mutex::new(PageState::fresh())));
                state.lock().await.users.clone()
            }
            #[cfg(feature = "csr")]
            {
                ListUsers {}.run_on_client().await.unwrap_or_default()
            }
        },
    );
    let filter = RwSignal::new(Filter::default());

    let filtered = Memo::new(move |_| {
        users
            .get()
            .map(|list| list.into_iter().filter(|u| filter.get().matches(u)).collect::<Vec<_>>())
            .unwrap_or_default()
    });

    let stats = Memo::new(move |_| {
        let all = users.get().unwrap_or_default();
        let total = all.len();
        let enabled = all.iter().filter(|u| u.enabled()).count();
        let fresh = all.iter().filter(|u| u.created_at.starts_with("2026-09")).count();
        let granted: i64 = all.iter().map(|u| u.quota).sum();
        let consumed: i64 = all.iter().map(|u| u.used_quota).sum();
        (total, enabled, fresh, cny(granted), cny(consumed))
    });

    view! {
        <h2>"统计"</h2>
        <div class="stats">
            <StatCard value=move || stats.get().0.to_string() label="总用户" />
            <StatCard value=move || stats.get().1.to_string() label="启用中" />
            <StatCard value=move || stats.get().2.to_string() label="本月新增" />
            <StatCard value=move || stats.get().3.clone() label="已发放额度" />
            <StatCard value=move || stats.get().4.clone() label="已消耗额度" />
            <Chips name="group" current=Arc::new(move || filter.get().group) options=group_options() on_pick=Arc::new(move |v| filter.update(|f| f.group = v)) />
            <Chips name="status" current=Arc::new(move || filter.get().status) options=status_options() on_pick=Arc::new(move |v| filter.update(|f| f.status = v)) />
            <Chips name="role" current=Arc::new(move || filter.get().role) options=role_options() on_pick=Arc::new(move |v| filter.update(|f| f.role = v)) />
        </div>

        <h2>"用户列表 " <span class="count">{move || format!("{} 人", filtered.get().len())}</span></h2>
        <div class="cards">
            {move || {
                filtered
                    .get()
                    .into_iter()
                    .map(|user| view! { <UserCard user=user users=users /> })
                    .collect_view()
            }}
        </div>
    }
}

fn group_options() -> Vec<(&'static str, &'static str)> {
    vec![("", "全部"), ("default", "default"), ("vip", "vip"), ("trial", "trial")]
}

fn status_options() -> Vec<(&'static str, &'static str)> {
    vec![("", "全部"), ("on", "启用"), ("off", "停用")]
}

fn role_options() -> Vec<(&'static str, &'static str)> {
    vec![("", "全部"), ("1", "普通用户"), ("10", "管理员"), ("100", "超级管理员")]
}

#[component]
fn StatCard(value: impl Fn() -> String + Send + 'static, label: &'static str) -> impl IntoView {
    view! {
        <div class="stat">
            <div class="stat-value">{value}</div>
            <div class="stat-label">{label}</div>
        </div>
    }
}

/// 一组筛选胶囊。点击只改客户端信号，不提交表单、不刷页面。
#[component]
fn Chips(
    name: &'static str,
    current: Arc<dyn Fn() -> String + Send + Sync>,
    options: Vec<(&'static str, &'static str)>,
    on_pick: Arc<dyn Fn(String) + Send + Sync>,
) -> impl IntoView {
    view! {
        <div class="chips" data-name=name>
            {options
                .into_iter()
                .map(|(value, label)| {
                    let cur = current.clone();
                    let pick = on_pick.clone();
                    view! {
                        <button
                            type="button"
                            class=move || if cur() == value { "chip on" } else { "chip" }
                            on:click=move |_| pick(value.to_string())
                        >
                            {label}
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}

#[component]
fn UserCard(user: User, users: Resource<Vec<User>>) -> impl IntoView {
    let user = RwSignal::new(user);
    let key = user.get().key;
    let toggle = ArcServerAction::<ToggleUser>::new();

    view! {
        <article class="card">
            <div class="card-head">
                <span class="name">{move || user.get().username.clone()}</span>
                <span class=move || if user.get().enabled() { "badge on" } else { "badge" }>
                    {move || if user.get().enabled() { "启用" } else { "停用" }}
                </span>
            </div>
            <div class="muted">{move || user.get().email.clone()}</div>
            <div class="meta">{move || format!("{} · {}", user.get().role_label(), user.get().groups.join(" / "))}</div>
            <div class="muted">{move || format!("额度 {} · 已用 {}", cny(user.get().quota), cny(user.get().used_quota))}</div>
            <div class="actions">
                <button type="button" class="act">"编辑"</button>
                <button type="button" class="act green">"充值"</button>
                <button
                    type="button"
                    class="act amber"
                    on:click=move |_| {
                        toggle.dispatch(ToggleUser { key: key.clone() });
                        users.refetch();
                    }
                >
                    {move || if user.get().enabled() { "停用" } else { "启用" }}
                </button>
            </div>
        </article>
    }
}

const STYLE: &str = r#"
:root { color-scheme: dark; }
* { box-sizing: border-box; }
body {
    margin: 0;
    background: #0b0e14;
    color: #e6e9f0;
    font: 13px/1.5 -apple-system, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
}
.shell { display: flex; min-height: 100vh; }
.rail {
    width: 44px;
    background: #11151d;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-top: 12px;
    gap: 14px;
    border-right: 1px solid #1c2230;
}
.dot {
    width: 10px; height: 10px; border-radius: 50%;
    background: #2a3142; cursor: pointer;
}
.dot.active { background: #60a5fa; }
.main { flex: 1; display: flex; flex-direction: column; min-width: 0; }
.tabs {
    display: flex; gap: 4px; padding: 10px 16px 0;
    border-bottom: 1px solid #1c2230;
    background: #0e1219;
}
.tab {
    padding: 8px 12px; color: #8b93a7; cursor: pointer;
    border-bottom: 2px solid transparent;
}
.tab.active { color: #e6e9f0; border-bottom-color: #60a5fa; }
.panel { margin: 16px; flex: 1; }
.panel-bar {
    height: 34px; padding: 0 14px;
    display: flex; align-items: center;
    background: #141925; color: #aab3c5;
    border: 1px solid #1c2230; border-bottom: none;
    border-radius: 10px 10px 0 0;
}
.panel-body {
    padding: 16px;
    background: #0f131c;
    border: 1px solid #1c2230;
    border-radius: 0 0 10px 10px;
}
.status {
    padding: 8px 16px; color: #5b6478; font-size: 12px;
    border-top: 1px solid #1c2230; background: #0e1219;
}
h2 { font-size: 15px; margin: 20px 0 10px; }
.count { color: #60a5fa; font-weight: 400; }
.stats { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 10px; }
.stat {
    background: #141925; border: 1px solid #1c2230;
    border-radius: 10px; padding: 12px 14px;
}
.stat-value { font-size: 20px; font-weight: 600; }
.stat-label { color: #8b93a7; margin-top: 2px; }
.filter {
    margin: 16px 0; padding: 14px;
    background: #141925; border: 1px solid #1c2230; border-radius: 10px;
}
.filter-title { color: #8b93a7; margin-bottom: 10px; }
.filter input {
    width: 100%; padding: 7px 10px;
    background: #0b0e14; color: #e6e9f0;
    border: 1px solid #232b3d; border-radius: 8px;
}
.chips { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 10px; }
.chip {
    padding: 4px 12px; border-radius: 999px;
    background: #0b0e14; color: #8b93a7;
    border: 1px solid #232b3d; cursor: pointer;
}
.chip.on { background: #1d2b45; color: #93c5fd; border-color: #2f4a7a; }
.cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 10px; }
.card {
    background: #141925; border: 1px solid #1c2230;
    border-radius: 10px; padding: 14px;
}
.card-head {
    display: flex; justify-content: space-between; align-items: center;
    margin-bottom: 6px;
}
.name { font-weight: 600; }
.badge {
    font-size: 12px; padding: 1px 8px; border-radius: 999px;
    background: #2a2118; color: #fbbf24;
}
.badge.on { background: #14261c; color: #34d399; }
.muted { color: #8b93a7; font-size: 12px; margin-top: 2px; }
.meta { margin-top: 6px; }
.actions { display: flex; gap: 6px; margin-top: 12px; }
.act {
    padding: 4px 12px; border-radius: 8px;
    background: #0b0e14; color: #aab3c5;
    border: 1px solid #232b3d; cursor: pointer;
}
.act.green { color: #34d399; }
.act.amber { color: #fbbf24; }
"#;
