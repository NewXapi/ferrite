//! 用户页布局：对齐 dioxus admin-page-users 的三区结构
//! （统计 / 筛选 / 卡片网格），视觉口径取 ui-components 的 Tailwind class 语义。
//! leptos 项目无 Tailwind，STYLE 里用等价 CSS 落同一套几何与配色。

use std::sync::Arc;

use leptos::prelude::*;
#[cfg(feature = "csr")]
use server_fn::ServerFn;

#[cfg(feature = "ssr")]
use leptos::config::LeptosOptions;
#[cfg(feature = "ssr")]
use leptos::hydration::{AutoReload, HydrationScripts};

use crate::pages::{
    AliasesPage, ChannelsPage, CurrencyPage, GatewayPage, GroupsPage, KeysPage, LeaderboardPage,
    ModelsPage, NetworkPage, OverviewPage, RedemptionsPage, RewardsPage, SessionsPage,
    SettingsPage, SubscriptionsPage, SystemPage, UsagePage,
};
#[cfg(feature = "ssr")]
use crate::users::PageState;
use crate::users::{Filter, ListUsers, ToggleUser, User, cny};

#[cfg(feature = "ssr")]
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
                <link rel="stylesheet" href="/assets/tailwind.css" />
                <style>{STYLE}</style>
                // ponytail: 标注 SDK 同源加载；grant 由 scripts/aino-leptos-grant.mjs
                // 签给 origin 8081，写到 site/assets/ainotation/connection.json。
                <script src="/assets/ainotation/ainotation.iife.js"></script>
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

const CARD_PAGE_SIZE: usize = 15;

#[component]
pub fn App() -> impl IntoView {
    // 0 总览 / 1 账户 / 2 管理。截图里 dioxus 停在总览。
    let section = RwSignal::new(0usize);
    let tab = RwSignal::new(0usize);
    let tabs = Memo::new(move |_| match section.get() {
        0 => vec!["总览", "模型", "排行榜"],
        1 => vec!["密钥·资料", "用量·日志", "邀请·奖励", "会话", "设置"],
        _ => vec![
            "网络",
            "用户",
            "分组",
            "别名",
            "渠道",
            "订阅",
            "兑换",
            "系统",
            "网关健康",
            "货币",
        ],
    });
    view! {
        <div class="shell">
            <Rail active=section on_select=move |i| { section.set(i); tab.set(0); } />
            <div class="main">
                <div class="top">
                    <nav class="tabs" aria-label="页面导航">
                        {move || tabs.get().into_iter().enumerate().map(|(i, label)| {
                            let on = tab.get() == i;
                            view! {
                                <button type="button" class=if on { "tab on" } else { "tab" }
                                    on:click=move |_| tab.set(i)>
                                    {label}
                                </button>
                            }
                        }).collect_view()}
                    </nav>
                </div>
                <main class="content">
                    {move || match (section.get(), tab.get()) {
                        (0, 0) => view! { <OverviewPage /> }.into_any(),
                        (0, 1) => view! { <ModelsPage /> }.into_any(),
                        (0, 2) => view! { <LeaderboardPage /> }.into_any(),
                        (1, 0) => view! { <KeysPage /> }.into_any(),
                        (1, 1) => view! { <UsagePage /> }.into_any(),
                        (1, 2) => view! { <RewardsPage /> }.into_any(),
                        (1, 3) => view! { <SessionsPage /> }.into_any(),
                        (1, 4) => view! { <SettingsPage /> }.into_any(),
                        (2, 0) => view! { <NetworkPage /> }.into_any(),
                        (2, 1) => view! { <UsersPage /> }.into_any(),
                        (2, 2) => view! { <GroupsPage /> }.into_any(),
                        (2, 3) => view! { <AliasesPage /> }.into_any(),
                        (2, 4) => view! { <ChannelsPage /> }.into_any(),
                        (2, 5) => view! { <SubscriptionsPage /> }.into_any(),
                        (2, 6) => view! { <RedemptionsPage /> }.into_any(),
                        (2, 7) => view! { <SystemPage /> }.into_any(),
                        (2, 8) => view! { <GatewayPage /> }.into_any(),
                        (2, 9) => view! { <CurrencyPage /> }.into_any(),
                        _ => view! { <EmptyPage /> }.into_any(),
                    }}
                </main>
                <div class="status">"admin_dev"</div>
            </div>
        </div>
    }
}

#[component]
fn EmptyPage() -> impl IntoView {
    view! { <div class="placeholder">"这个页还没搬"</div> }
}

#[component]
fn Rail(
    active: RwSignal<usize>,
    on_select: impl Fn(usize) + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let labels = ["总览", "账户", "管理"];
    view! {
        <aside class="rail" aria-label="主导航">
            <nav class="rail-nav">
                {(0..3).map(|i| {
                    let pick = on_select.clone();
                    let on = move || active.get() == i;
                    view! {
                        <button type="button" class=move || if on() { "rail-btn on" } else { "rail-btn" }
                            aria-label=labels[i]
                            on:click=move |_| pick(i)>
                            {rail_icon(i)}
                            <span class="tip">{labels[i]}</span>
                        </button>
                    }
                }).collect_view()}
            </nav>
        </aside>
    }
}

fn rail_icon(i: usize) -> impl IntoView {
    let path = match i {
        0 => "M3 9v7m0 0h7m-7 0v3m7-3V9a3 3 0 0 1 6 0v7m-6 0h6",
        1 => "M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2",
        _ => "",
    };
    view! {
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor"
            stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            {if i == 1 {
                view! { <circle cx="12" cy="7" r="4"></circle> }.into_any()
            } else if i == 2 {
                view! {
                    <circle cx="12" cy="12" r="3"></circle>
                    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h.01a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51h.01a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v.01a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
                }.into_any()
            } else {
                view! { <path d=path></path> }.into_any()
            }}
        </svg>
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
                    .unwrap_or_else(|| {
                        std::sync::Arc::new(tokio::sync::Mutex::new(PageState::fresh()))
                    });
                state.lock().await.users.clone()
            }
            #[cfg(feature = "csr")]
            {
                ListUsers {}.run_on_client().await.unwrap_or_default()
            }
        },
    );
    let filter = RwSignal::new(Filter::default());
    let search = RwSignal::new(String::new());
    let page = RwSignal::new(0usize);

    let filtered = Memo::new(move |_| {
        users
            .get()
            .map(|list| {
                list.into_iter()
                    .filter(|u| filter.get().matches(u))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    });

    let stats = Memo::new(move |_| {
        let all = users.get().unwrap_or_default();
        let total = all.len();
        let enabled = all.iter().filter(|u| u.enabled()).count();
        let fresh = all
            .iter()
            .filter(|u| u.created_at.starts_with("2026-09"))
            .count();
        let granted: i64 = all.iter().map(|u| u.quota).sum();
        let consumed: i64 = all.iter().map(|u| u.used_quota).sum();
        (total, enabled, fresh, cny(granted), cny(consumed))
    });

    // 筛选后条数变小：clamp 到最后一页，不落空页（dioxus 同款策略）。
    let editing = RwSignal::new(None::<String>);
    let visible = Memo::new(move |_| {
        let all = filtered.get();
        let pages = all.len().div_ceil(CARD_PAGE_SIZE).max(1);
        let clamped = page.get().min(pages - 1);
        let start = clamped * CARD_PAGE_SIZE;
        all[start..(start + CARD_PAGE_SIZE).min(all.len())].to_vec()
    });

    view! {
        <div class="users-panel" data-testid="users-panel">
            // 1. 统计区：手机 1 栏 / 平板 3 栏 / 大屏 5 栏
            <section class="sec" id="users-sec-stats">
                <h2>"用户概览"</h2>
                <div class="stats">
                    <StatCard value=move || stats.get().0.to_string() label="总用户" />
                    <StatCard value=move || stats.get().1.to_string() label="启用中" />
                    <StatCard value=move || stats.get().2.to_string() label="本月新增" />
                    <StatCard value=move || stats.get().3.clone() label="总发放额度" />
                    <StatCard value=move || stats.get().4.clone() label="总消耗" />
                </div>
            </section>

            // 2. 筛选区
            <section class="sec filter" id="users-sec-filter">
                <div class="filter-head">
                    <h2 class="filter-title">"筛选用户"</h2>
                    <div class="filter-btns">
                        <button class="btn-ghost" data-testid="refresh-users"
                            type="button"
                            on:click=move |_| users.refetch()>
                            "刷新"
                        </button>
                        <button class="btn-primary" data-testid="new-user" type="button"
                            on:click=move |_| editing.set(Some(String::new()))>
                            "✚ 新建用户"
                        </button>
                    </div>
                </div>

                <input class="search" type="text" placeholder="搜索用户名或邮箱"
                    data-testid="users-search"
                    prop:value=move || search.get()
                    on:input=move |e| search.set(leptos::leptos_dom::helpers::event_target_value(&e)) />

                <div class="segments">
                    <Chips name="group" current=Arc::new(move || filter.get().group)
                        options=group_options() on_pick=Arc::new(move |v| filter.update(|f| f.group = v)) />
                    <Chips name="status" current=Arc::new(move || filter.get().status)
                        options=status_options() on_pick=Arc::new(move |v| filter.update(|f| f.status = v)) />
                    <Chips name="role" current=Arc::new(move || filter.get().role)
                        options=role_options() on_pick=Arc::new(move |v| filter.update(|f| f.role = v)) />
                </div>
            </section>

            // 3. 用户卡片网格
            <section class="sec" id="users-sec-list">
                <div class="sec-head">
                    <h2>"用户列表"</h2>
                    <div class="sec-trailing">
                        <span class="badge">{move || format!("{} 人", filtered.get().len())}</span>
                        <Pager total=move || filtered.get().len() page=page />
                    </div>
                </div>
                <UserList users=users visible=visible editing=editing />
            </section>
            {move || editing.get().map(|key| view! { <EditDialog key=key close=editing /> })}
        </div>
    }
}

fn group_options() -> Vec<(&'static str, &'static str)> {
    vec![
        ("", "全部"),
        ("default", "default"),
        ("vip", "vip"),
        ("trial", "trial"),
    ]
}

fn status_options() -> Vec<(&'static str, &'static str)> {
    vec![("", "全部"), ("on", "启用"), ("off", "停用")]
}

fn role_options() -> Vec<(&'static str, &'static str)> {
    vec![
        ("", "全部"),
        ("1", "普通用户"),
        ("10", "管理员"),
        ("100", "超级管理员"),
    ]
}

#[component]
fn StatCard(value: impl Fn() -> String + Send + 'static, label: &'static str) -> impl IntoView {
    view! {
        <div class="stat">
            <p class="stat-value">{value}</p>
            <p class="stat-label">{label}</p>
        </div>
    }
}

/// 列表区：加载中 / 空态 / 卡片网格三态，抽出独立组件避免 match 类型链过深。
#[component]
fn UserList(
    users: Resource<Vec<User>>,
    visible: Memo<Vec<User>>,
    editing: RwSignal<Option<String>>,
) -> impl IntoView {
    move || match users.get() {
        None => view! {
            <div class="placeholder" role="status">"正在加载用户…"</div>
        }
        .into_any(),
        Some(list) if list.is_empty() => view! {
            <div class="placeholder" role="status">"没有匹配的用户"</div>
        }
        .into_any(),
        Some(_) => view! {
            <div class="cards" role="list" aria-label="用户列表">
                {move || {
                    visible
                        .get()
                        .into_iter()
                        .map(|user| view! { <UserCard user=user users=users editing=editing /> })
                        .collect_view()
                }}
            </div>
        }
        .into_any(),
    }
}

/// 一组筛选胶囊。点击只改客户端信号，不提交表单、不刷页面。
/// 对齐 dioxus SegmentedCapsule：圆角分段条，选中项白底。
#[component]
fn Chips(
    name: &'static str,
    current: Arc<dyn Fn() -> String + Send + Sync>,
    options: Vec<(&'static str, &'static str)>,
    on_pick: Arc<dyn Fn(String) + Send + Sync>,
) -> impl IntoView {
    view! {
        <div class="chips" data-name=name role="group" aria-label=name>
            {options
                .into_iter()
                .map(|(value, label)| {
                    let cur = current.clone();
                    let cur2 = current.clone();
                    let pick = on_pick.clone();
                    view! {
                        <button
                            type="button"
                            role="tab"
                            aria-selected=move || cur() == value
                            class=move || if cur2() == value { "chip on" } else { "chip" }
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

/// 分页器：总条数不超过一页时不渲染；当前页白底高亮。
#[component]
fn Pager(
    total: impl Fn() -> usize + Send + Sync + 'static,
    page: RwSignal<usize>,
) -> impl IntoView {
    let pages = Memo::new(move |_| total().div_ceil(CARD_PAGE_SIZE).max(1));
    let current = Memo::new(move |_| page.get().min(pages.get() - 1));

    view! {
        <nav class="pager" role="navigation" aria-label="分页">
            {move || {
                let n = pages.get();
                let cur = current.get();
                (0..n).map(move |i| {
                    let on = i == cur;
                    view! {
                        <button type="button"
                            class=if on { "pg-btn on" } else { "pg-btn" }
                            aria-current=if on { "page" } else { "false" }
                            disabled=on
                            on:click=move |_| page.set(i)>
                            {i + 1}
                        </button>
                    }
                }).collect_view()
            }}
        </nav>
    }
}

/// 用户卡：对齐 dioxus ui::UserCard 的三页签结构
/// （基本信息 / 额度 / 系统），外壳带文字页签切换。
#[component]
fn UserCard(
    user: User,
    users: Resource<Vec<User>>,
    editing: RwSignal<Option<String>>,
) -> impl IntoView {
    let user = RwSignal::new(user);
    let key = user.get().key;
    let edit_key = key.clone();
    let tab = RwSignal::new(0usize);
    let toggle = ArcServerAction::<ToggleUser>::new();
    let tabs = ["基本信息", "额度", "系统"];

    view! {
        <article class="card" role="region" aria-label=move || user.get().username.clone()>
            <div class="card-head">
                <div class="card-title">
                    <h3 class="name">{move || user.get().username.clone()}</h3>
                    <p class="muted email">{move || user.get().email.clone()}</p>
                </div>
                <div class="card-tabs" role="tablist" aria-label="内容页签">
                    {(0..tabs.len()).map(|i| {
                        let active = move || tab.get() == i;
                        view! {
                            <button type="button" role="tab"
                                class=move || if active() { "ct on" } else { "ct" }
                                aria-selected=active
                                on:click=move |_| tab.set(i)>
                                {tabs[i]}
                            </button>
                        }
                    }).collect_view()}
                </div>
            </div>

            <div class="card-body">
                // ponytail: 三页签同格叠加，容器高度取最高者，切页签不跳动。
                <div class=move || if tab.get() == 0 { "panel on" } else { "panel off" }>
                    <div class="kv"><span>"用户名"</span><span class="val">{move || user.get().username.clone()}</span></div>
                    <div class="kv"><span>"邮箱"</span><span class="val">{move || user.get().email.clone()}</span></div>
                    <div class="kv"><span>"角色"</span><span class="val">{move || user.get().role_label()}</span></div>
                    <div class="kv"><span>"状态"</span><span class="val">{move || if user.get().enabled() { "启用" } else { "停用" }}</span></div>
                    <div class="groups">
                        <p class="groups-label">"分组"</p>
                        <div class="group-list">
                            {move || {
                                let g = user.get().groups.clone();
                                if g.is_empty() {
                                    vec![view! { <span class="group-empty">"无分组"</span> }.into_any()]
                                } else {
                                    g.into_iter()
                                        .map(|name| view! { <span class="group-chip">{name}</span> }.into_any())
                                        .collect()
                                }
                            }}
                        </div>
                        <button type="button" class="act" data-testid="user-edit"
                            on:click=move |_| editing.set(Some(edit_key.clone()))>
                            "编辑"
                        </button>
                        <button type="button" class="act green" data-testid="user-topup">"充值"</button>
                        <button type="button" class="act amber" data-testid="user-toggle"
                            on:click=move |_| {
                                toggle.dispatch(ToggleUser { key: key.clone() });
                                users.refetch();
                            }>
                            {move || if user.get().enabled() { "停用" } else { "启用" }}
                        </button>
                    </div>
                </div>
                <div class=move || if tab.get() == 1 { "panel on" } else { "panel off" }>
                    <div class="kv"><span>"已用"</span><span class="val">{move || cny(user.get().used_quota)}</span></div>
                    <div class="kv"><span>"总额"</span><span class="val">{move || cny(user.get().quota)}</span></div>
                    <div class="bar">
                        <div class="bar-fill"
                            style:width=move || format!("{:.1}%",
                                if user.get().quota > 0 {
                                    (user.get().used_quota as f64 / user.get().quota as f64 * 100.0).min(100.0)
                                } else { 0.0 })>
                        </div>
                    </div>
                </div>
                <div class=move || if tab.get() == 2 { "panel on" } else { "panel off" }>
                    <div class="kv"><span>"Key"</span><span class="val mono">{move || user.get().key.chars().take(8).collect::<String>()}</span></div>
                    <div class="kv"><span>"创建"</span><span class="val">{move || user.get().created_at.clone()}</span></div>
                </div>
            </div>
        </article>
    }
}

/// 编辑弹窗。key 为空是新建。字段先只展示，保存还没接后端。
#[component]
fn EditDialog(key: String, close: RwSignal<Option<String>>) -> impl IntoView {
    let title = if key.is_empty() {
        "新建用户"
    } else {
        "编辑用户"
    };
    view! {
        <div class="modal" role="dialog" aria-label=title>
            <div class="modal-card">
                <div class="modal-head">
                    <h3>{title}</h3>
                    <button type="button" class="btn-ghost" on:click=move |_| close.set(None)>"取消"</button>
                </div>
                <label>"用户名" <input class="search" type="text" /></label>
                <label>"邮箱" <input class="search" type="text" /></label>
                <label>"角色" <input class="search" type="text" /></label>
                <div class="modal-actions">
                    <button type="button" class="btn-primary">"保存"</button>
                </div>
            </div>
        </div>
    }
}
const STYLE: &str = r#"
:root { color-scheme: dark; }
* { box-sizing: border-box; }
body { margin: 0; background: #09090b; color: #e4e4e7;
    font: 13px/1.5 -apple-system, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif; }
.shell { display: flex; height: 100vh; overflow: hidden; background: #09090b; color: #fafafa; }
.rail {
    width: 56px; flex: 0 0 56px; height: 100vh;
    display: flex; flex-direction: column; align-items: center;
    border-right: 1px solid #27272a; background: #09090b; padding: 12px 0;
}
.rail-nav { display: flex; flex-direction: column; align-items: center; gap: 4px; }
.rail-btn {
    position: relative; width: 36px; height: 36px; padding: 0;
    display: flex; align-items: center; justify-content: center;
    border: 0; border-radius: 8px; background: transparent; color: #71717a; cursor: pointer;
}
.rail-btn:hover { background: #18181b; color: #e4e4e7; }
.rail-btn.on { background: #27272a; color: #fafafa; }
.tip {
    pointer-events: none; position: absolute; left: 100%; top: 50%; transform: translateY(-50%);
    margin-left: 8px; white-space: nowrap; opacity: 0;
    border: 1px solid #3f3f46; background: #18181b; color: #e4e4e7;
    border-radius: 6px; padding: 4px 8px; font-size: 12px;
}
.rail-btn:hover .tip { opacity: 1; }
.main { flex: 1; min-width: 0; display: flex; flex-direction: column; }
.top { flex: 0 0 auto; display: flex; padding: 12px 16px 0; }
.tabs { display: flex; align-items: center; gap: 4px; overflow-x: auto; }
.tab {
    position: relative; height: 24px; flex: 0 0 auto; padding: 0 8px;
    border: 0; background: transparent; cursor: pointer;
    font: 500 14px/1 inherit; color: #71717a;
}
.tab:hover { color: #d4d4d8; }
.tab.on { color: #fafafa; }
.tab.on::after {
    content: ""; position: absolute; left: 8px; right: 8px; bottom: 0;
    height: 2px; border-radius: 999px; background: #fafafa;
}
.content { flex: 1; min-height: 0; overflow: auto; padding: 16px 24px 16px; }
.status { flex: 0 0 auto; padding: 4px 16px 6px; font-size: 12px; color: #a1a1aa; }

/* —— 用户页三区 —— */
.users-panel { display: flex; flex-direction: column; gap: 24px; }
.sec { display: flex; flex-direction: column; gap: 12px; }
.sec h2, .filter-title { font-size: 15px; font-weight: 500; color: #fafafa; margin: 0; }
.sec-head { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 8px; }
.sec-trailing { display: flex; align-items: center; gap: 8px; }
.badge { border-radius: 999px; background: #27272a; padding: 2px 12px; font-size: 12px; color: #a1a1aa; }

/* 统计区：手机 1 栏 / 平板 3 栏 / 大屏 5 栏 */
.stats { display: grid; grid-template-columns: 1fr; gap: 12px; }
@media (min-width: 768px) { .stats { grid-template-columns: repeat(3, 1fr); } }
@media (min-width: 1024px) { .stats { grid-template-columns: repeat(5, 1fr); } }
.stat {
    border: 1px solid #27272a; background: #18181b;
    border-radius: 12px; padding: 12px 16px;
    transition: border-color .15s;
}
.stat:hover { border-color: #3f3f46; }
.stat-value { margin: 0; font-size: 20px; font-weight: 600; letter-spacing: -0.02em; color: #fff; }
.stat-label { margin: 2px 0 0; font-size: 12px; color: #71717a; }

/* 筛选区 */
.filter {
    gap: 16px;
    border: 1px solid #27272a; background: #18181b;
    border-radius: 12px; padding: 20px;
}
.filter-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.filter-btns { display: flex; gap: 8px; }
.btn-ghost {
    flex-shrink: 0; border-radius: 10px; border: 1px solid #3f3f46;
    padding: 6px 12px; font-size: 12px; color: #d4d4d8;
    background: transparent; cursor: pointer; transition: background .15s;
}
.btn-ghost:hover { background: #27272a; }
.btn-primary {
    flex-shrink: 0; border-radius: 10px; border: none;
    padding: 6px 16px; font-size: 12px; font-weight: 500;
    background: #fff; color: #18181b; cursor: pointer; transition: background .15s;
}
.btn-primary:hover { background: #e4e4e7; }
.search {
    width: 100%; padding: 8px 16px;
    background: #09090b; color: #fafafa;
    border: 1px solid #3f3f46; border-radius: 10px;
    outline: none; font: inherit; transition: border-color .15s;
}
.search:focus { border-color: #71717a; }
.search::placeholder { color: #52525b; }
.segments { display: flex; flex-direction: column; gap: 12px; }

/* 胶囊分段：贴内容宽度，不拉满整行。 */
.chips {
    display: inline-flex; width: fit-content; overflow: hidden;
    border-radius: 999px; border: 1px solid #3f3f46; background: #09090b;
}
.chip {
    flex: 0 0 auto;
    border-right: 1px solid #27272a;
    padding: 4px 12px; text-align: center;
    font-size: 12px; color: #71717a;
    background: transparent; cursor: pointer; transition: background .15s;
}
.chip:last-child { border-right: none; }
.chip:hover { background: #27272a; color: #e4e4e7; }
.chip.on { background: #f4f4f5; color: #18181b; font-weight: 500; }

/* 卡片网格：1 / 3 / 5 列 */
.cards { display: grid; grid-template-columns: 1fr; gap: 12px; }
@media (min-width: 768px) { .cards { grid-template-columns: repeat(3, 1fr); } }
@media (min-width: 1024px) { .cards { grid-template-columns: repeat(5, 1fr); } }
.placeholder {
    border: 1px dashed #3f3f46; background: #18181b;
    border-radius: 12px; padding: 64px 16px; text-align: center; color: #71717a;
}

/* 用户卡（AdminCard 外壳） */
.card {
    display: flex; flex-direction: column; justify-content: space-between;
    border: 1px solid #27272a; background: #18181b;
    border-radius: 12px; padding: 16px;
    transition: border-color .2s, background .2s;
}
.card:hover { border-color: #52525b; background: #1f1f23; }
.card-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; }
.card-title { min-width: 0; flex: 1; }
.card-tabs { display: flex; gap: 2px; }
.ct {
    position: relative; padding: 2px 6px; font-size: 11px; cursor: pointer;
    background: transparent; border: 0; color: #71717a;
}
.ct.on { color: #fafafa; }
.ct.on::after {
    content: ""; position: absolute; left: 6px; right: 6px; bottom: -2px;
    height: 2px; background: #fafafa;
}
.name { margin: 0; font-size: 14px; font-weight: 500; color: #fafafa;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.email { margin: 2px 0 0; font-size: 11px; color: #71717a;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.card-body { margin-top: 12px; display: grid; grid-template-columns: 1fr; }
.panel.on { grid-column: 1; grid-row: 1; }
.panel.off { grid-column: 1; grid-row: 1; visibility: hidden; pointer-events: none; }
.kv { display: flex; justify-content: space-between; gap: 8px; font-size: 12px; }
.kv > span:first-child { color: #71717a; }
.kv .val { font-weight: 500; color: #e4e4e7; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.mono { font-family: ui-monospace, "SF Mono", Menlo, monospace; }
.groups { margin-top: 10px; }
.groups-label { margin: 0 0 6px; font-size: 11px; color: #71717a; }
.group-list { display: flex; flex-wrap: wrap; gap: 6px; }
.group-chip {
    border: 1px solid #3f3f46; background: #27272a;
    border-radius: 999px; padding: 1px 8px; font-size: 11px; color: #d4d4d8;
}
.group-empty { font-size: 11px; color: #52525b; }
.actions { display: flex; gap: 6px; margin-top: 16px; padding-top: 12px; border-top: 1px solid #27272a; }
.act {
    flex: 1; border-radius: 8px; border: 1px solid #3f3f46;
    background: #27272a; padding: 6px 0; font-size: 12px; font-weight: 500;
    color: #d4d4d8; cursor: pointer; transition: background .15s;
}
.act:hover { background: #3f3f46; color: #fff; }
.act.green { color: #34d399; }
.act.green:hover { color: #6ee7b7; }
.act.amber { color: #fbbf24; }
.act.amber:hover { color: #fcd34d; }

/* 分页器 */
.pg-btn {
    border-radius: 8px; border: 1px solid #3f3f46;
    padding: 2px 8px; font-size: 11px; color: #d4d4d8;
    background: transparent; cursor: pointer; transition: background .15s;
}
.pg-btn:hover:not(:disabled) { background: #27272a; }
.pg-btn:disabled { opacity: .4; cursor: default; }
.pg-btn.on { background: #fff; border-color: #fff; font-weight: 500; color: #18181b; }
.modal {
    position: fixed; inset: 0; z-index: 50;
    display: flex; align-items: center; justify-content: center;
    background: rgba(0,0,0,.55);
}
.modal-card {
    width: min(420px, calc(100vw - 32px));
    display: flex; flex-direction: column; gap: 12px;
    border: 1px solid #3f3f46; background: #18181b;
    border-radius: 12px; padding: 16px;
}
.modal-head { display: flex; align-items: center; justify-content: space-between; }
.modal-head h3 { margin: 0; font-size: 14px; }
.modal-card label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: #a1a1aa; }
.modal-actions { display: flex; justify-content: flex-end; }
.bar-fill { height: 100%; border-radius: 999px; background: #34d399; transition: width .2s; }
"#;
