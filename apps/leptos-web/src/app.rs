//! 复刻 admin-web 的控制台壳 + 用户页。SSR-only：筛选是带 query 的链接，
//! 启停是表单 POST，浏览器不跑任何 wasm。样式内联在 shell 里，无构建步骤。

use leptos::prelude::*;
use leptos::server_fn::ServerFn;
use leptos_router::location::RequestUrl;

use crate::users::{cny, PageState, ToggleUser, User};

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
    let tabs = ["网络", "用户", "分组", "别名", "渠道", "订阅", "兑换", "系统", "网关健康", "货币"];
    view! {
        <nav class="tabs">
            {tabs
                .into_iter()
                .map(|t| {
                    view! { <span class=if t == "用户" { "tab active" } else { "tab" }>{t}</span> }
                })
                .collect_view()}
        </nav>
    }
}

/// 当前请求的筛选条件，从 URL query 读。
#[derive(Clone)]
struct Filter {
    query: String,
    group: String,
    status: String,
    role: String,
}
#[component]
fn UsersPage() -> impl IntoView {
    let url = use_context::<RequestUrl>();
    let params = url.as_ref().and_then(|url| url.parse().ok());
    let param = |name: &str| {
        params
            .as_ref()
            .and_then(|url| url.search_params().get(name))
            .unwrap_or_default()
    };
    let filter = Filter {
        query: param("q"),
        group: param("group"),
        status: param("status"),
        role: param("role"),
    };
    let state = use_context::<std::sync::Arc<tokio::sync::Mutex<PageState>>>()
        .unwrap_or_else(|| std::sync::Arc::new(tokio::sync::Mutex::new(PageState::fresh())));

    let page = PageState {
        users: Vec::new(),
        query: filter.query.clone(),
        group: filter.group.clone(),
        status: filter.status.clone(),
        role: filter.role.clone(),
    };
    // 列表在渲染时同步读取；toggle 之后整页重渲染，数字是新鲜的。
    let users = state.try_lock().map(|g| g.users.clone()).unwrap_or_default();
    let view_state = PageState { users: users.clone(), ..page };
    let filtered = view_state.filtered();

    let total = users.len();
    let enabled = users.iter().filter(|u| u.enabled()).count();
    let fresh = users.iter().filter(|u| u.created_at.starts_with("2026-09")).count();
    let granted: i64 = users.iter().map(|u| u.quota).sum();
    let consumed: i64 = users.iter().map(|u| u.used_quota).sum();

    let stats = [
        (total.to_string(), "总用户"),
        (enabled.to_string(), "启用中"),
        (fresh.to_string(), "本月新增"),
        (cny(granted), "已发放额度"),
        (cny(consumed), "已消耗额度"),
    ];

    let kept = filter.clone();
    view! {
        <h2>"统计"</h2>
        <div class="stats">
            {stats
                .into_iter()
                .map(|(value, label)| view! { <StatCard value=value label=label /> })
                .collect_view()}
        </div>

        <div class="filter">
            <div class="filter-title">"筛选"</div>
            <form method="get" action="/">
                <input name="q" value=kept.query.clone() placeholder="搜索用户名或邮箱" />
                <Chips name="group" current=kept.group.clone() options=group_options() kept=kept.clone() />
                <Chips name="status" current=kept.status.clone() options=status_options() kept=kept.clone() />
                <Chips name="role" current=kept.role.clone() options=role_options() kept=kept.clone() />
            </form>
        </div>

        <h2>"用户列表 " <span class="count">{format!("{} 人", filtered.len())}</span></h2>
        <div class="cards">
            {filtered
                .into_iter()
                .map(|user| view! { <UserCard user=user /> })
                .collect_view()}
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
fn StatCard(value: String, label: &'static str) -> impl IntoView {
    view! {
        <div class="stat">
            <div class="stat-value">{value}</div>
            <div class="stat-label">{label}</div>
        </div>
    }
}

/// 一组筛选胶囊。隐藏字段保留搜索词和其他筛选，避免点一个胶囊丢掉其余条件。
#[component]
fn Chips(
    name: &'static str,
    current: String,
    options: Vec<(&'static str, &'static str)>,
    kept: Filter,
) -> impl IntoView {
    let hidden = [("q", kept.query), ("group", kept.group), ("status", kept.status), ("role", kept.role)]
        .into_iter()
        .filter(|(key, _)| *key != name)
        .map(|(key, value)| view! { <input type="hidden" name=key value=value /> })
        .collect_view();
    view! {
        <div class="chips">
            {hidden}
            {options
                .into_iter()
                .map(|(value, label)| {
                    let on = current == value;
                    view! {
                        <button
                            type="submit"
                            name=name
                            value=value
                            class=if on { "chip on" } else { "chip" }
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
fn UserCard(user: User) -> impl IntoView {
    let badge = if user.enabled() { "启用" } else { "停用" };
    let toggle_label = if user.enabled() { "停用" } else { "启用" };
    let groups = user.groups.join(" / ");
    let quota = cny(user.quota);
    let used = cny(user.used_quota);
    let key = user.key.clone();

    view! {
        <article class="card">
            <div class="card-head">
                <span class="name">{user.username.clone()}</span>
                <span class=if user.enabled() { "badge on" } else { "badge" }>{badge}</span>
            </div>
            <div class="muted">{user.email.clone()}</div>
            <div class="meta">{format!("{} · {}", user.role_label(), groups)}</div>
            <div class="muted">{format!("额度 {} · 已用 {}", quota, used)}</div>
            <div class="actions">
                <button type="button" class="act">"编辑"</button>
                <button type="button" class="act green">"充值"</button>
                <form method="post" action=ToggleUser::PATH>
                    <input type="hidden" name="key" value=key />
                    <button type="submit" class="act amber">{toggle_label}</button>
                </form>
            </div>
        </article>
    }
}

const STYLE: &str = r#"
:root { color-scheme: dark; }
* { box-sizing: border-box; }
body { margin: 0; background: #09090b; color: #f4f4f5; font: 14px/1.5 sans-serif; }
.shell { display: flex; height: 100vh; padding: 12px; gap: 12px; }
.rail { display: flex; flex-direction: column; justify-content: center; gap: 10px; }
.dot { width: 8px; height: 8px; border-radius: 99px; background: #3f3f46; }
.dot.active { height: 36px; background: #d4d4d8; }
.main { flex: 1; display: flex; flex-direction: column; min-width: 0; }
.tabs { display: flex; gap: 4px; height: 36px; align-items: center; overflow-x: auto; }
.tab { padding: 0 10px; color: #71717a; white-space: nowrap; }
.tab.active { color: #f4f4f5; border-bottom: 2px solid #f4f4f5; }
.panel { flex: 1; min-height: 0; display: flex; flex-direction: column; border: 1px solid #27272a; border-radius: 16px; background: rgba(24,24,27,.6); }
.panel-bar { height: 36px; display: flex; align-items: center; padding: 0 16px; border-bottom: 1px solid #27272a; color: #71717a; font-size: 12px; }
.panel-body { flex: 1; overflow: auto; padding: 20px; }
.status { height: 28px; display: flex; align-items: center; color: #71717a; font-size: 12px; }
h2 { font-size: 16px; font-weight: 500; margin: 0 0 12px; }
.count { color: #a1a1aa; font-size: 12px; }
.stats { display: flex; flex-wrap: wrap; gap: 12px; margin-bottom: 24px; }
.stat { width: 180px; padding: 16px; border: 1px solid #27272a; border-radius: 12px; background: #18181b; }
.stat-value { font-size: 20px; font-weight: 600; }
.stat-label { font-size: 12px; color: #a1a1aa; margin-top: 4px; }
.filter { border: 1px solid #27272a; border-radius: 12px; background: #18181b; padding: 20px; margin-bottom: 24px; }
.filter-title { color: #d4d4d8; margin-bottom: 12px; }
input[name=q] { width: 100%; background: #09090b; color: #f4f4f5; border: 1px solid #3f3f46; border-radius: 12px; padding: 10px 16px; }
.chips { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 10px; }
.chip { background: transparent; color: #d4d4d8; border: 1px solid #3f3f46; border-radius: 99px; padding: 3px 12px; cursor: pointer; font-size: 12px; }
.chip.on { background: #f4f4f5; color: #09090b; }
.cards { display: flex; flex-wrap: wrap; gap: 12px; }
.card { width: 280px; padding: 16px; border: 1px solid #27272a; border-radius: 16px; background: #18181b; }
.card-head { display: flex; justify-content: space-between; align-items: center; }
.name { font-weight: 600; }
.badge { font-size: 11px; color: #a1a1aa; border: 1px solid rgba(161,161,170,.5); border-radius: 99px; padding: 1px 8px; }
.badge.on { color: #34d399; border-color: rgba(52,211,153,.5); }
.muted { color: #a1a1aa; font-size: 12px; }
.meta { color: #d4d4d8; font-size: 12px; margin-top: 12px; }
.actions { display: flex; gap: 4px; border-top: 1px solid #27272a; margin-top: 12px; padding-top: 8px; }
.actions form { flex: 1; display: flex; }
.act { flex: 1; width: 100%; background: transparent; border: none; color: #d4d4d8; cursor: pointer; font-size: 12px; padding: 6px 0; }
.act.green { color: #34d399; }
.act.amber { color: #fbbf24; }
"#;
