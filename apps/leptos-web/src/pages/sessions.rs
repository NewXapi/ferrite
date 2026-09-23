use leptos::prelude::*;

use crate::ui::{Button, CardGrid, Dialog, DialogTrigger};

#[derive(Clone, Debug, PartialEq)]
pub struct SessionData {
    pub sid: String,
    pub user_agent: String,
    pub ip: String,
    pub login_method: String,
    pub last_active: String,
    pub expires_at: String,
    pub current: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum SessionStatus {
    Active,
    Inactive,
}

fn get_static_sessions() -> Vec<SessionData> {
    vec![
        SessionData {
            sid: "sess_001".to_string(),
            user_agent: "Chrome 120.0.0.0 on Windows 10".to_string(),
            ip: "192.168.1.100".to_string(),
            login_method: "密码".to_string(),
            last_active: "2024-01-15 10:30:00".to_string(),
            expires_at: "2024-01-15 23:59:00".to_string(),
            current: true,
        },
        SessionData {
            sid: "sess_002".to_string(),
            user_agent: "Safari 17.0.0 on macOS 14".to_string(),
            ip: "10.0.0.5".to_string(),
            login_method: "Google".to_string(),
            last_active: "2024-01-14 15:45:00".to_string(),
            expires_at: "2024-01-15 10:00:00".to_string(),
            current: false,
        },
        SessionData {
            sid: "sess_003".to_string(),
            user_agent: "Firefox 119.0 on Linux".to_string(),
            ip: "172.16.0.20".to_string(),
            login_method: "GitHub".to_string(),
            last_active: "2024-01-13 08:15:00".to_string(),
            expires_at: "2024-01-14 18:00:00".to_string(),
            current: false,
        },
        SessionData {
            sid: "sess_004".to_string(),
            user_agent: "Edge 120.0.0.0 on Android".to_string(),
            ip: "192.168.0.50".to_string(),
            login_method: "邮箱".to_string(),
            last_active: "2024-01-12 20:00:00".to_string(),
            expires_at: "2024-01-13 16:00:00".to_string(),
            current: false,
        },
    ]
}

#[component]
fn SessionCard(session: SessionData, busy: bool, on_revoke: Callback<String>) -> impl IntoView {
    let is_current = session.current;
    let sid = session.sid.clone();
    let ua = session.user_agent.clone();
    // `title` 属性与正文各消费一次：属性用克隆副本，正文用原值。
    let ua_for_title = ua.clone();
    let ip = session.ip.clone();
    let login_method = session.login_method.clone();
    let last_active = session.last_active.clone();
    let expires_at = session.expires_at.clone();

    view! {
        <div class="rounded-xl border border-zinc-800 bg-zinc-900/60 p-4">
            <div class="flex items-start justify-between gap-3">
                <div class="min-w-0">
                    <div class="flex items-center gap-2">
                        <p class="truncate text-sm text-zinc-200" title=ua_for_title.clone()>
                            {ua}
                        </p>
                        {is_current.then(|| view! {
                            <span class="shrink-0 rounded-full bg-emerald-500/20 px-2 py-0.5 text-[10px] font-medium text-emerald-400">
                                "当前设备"
                            </span>
                        })}
                    </div>
                    <div class="mt-2 grid grid-cols-1 gap-1 text-xs text-zinc-500 sm:grid-cols-2">
                        <span>"IP: " {ip}</span>
                        <span>"登录方式: " {login_method}</span>
                        <span>"最后活跃: " {last_active}</span>
                        <span>"到期: " {expires_at}</span>
                    </div>
                </div>
                <Button
                    button_type="button"
                    variant="destructive"
                    class="shrink-0 text-red-400"
                    disabled=busy
                    attr:data-testid=format!("revoke-session-{}", sid)
                    attr:aria-label="吊销此会话"
                    on:click=move |_| on_revoke.run(sid.clone())
                >
                    "吊销"
                </Button>
            </div>
        </div>
    }
}

#[component]
fn RevokeConfirmModal(
    open: RwSignal<bool>,
    on_confirm: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    view! {
        <Dialog open=open>
            <DialogTrigger slot>
                <span class="hidden">"确认"</span>
            </DialogTrigger>
            <div
                class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm"
                role="dialog"
                aria-modal="true"
                aria-label="吊销当前设备确认"
                data-testid="confirm-revoke-current-modal"
            >
                <div class="w-full max-w-md rounded-2xl border border-red-500/40 bg-zinc-900 p-5 shadow-xl">
                    <h3 class="text-base font-semibold text-zinc-100">"吊销当前设备"</h3>
                    <p class="mt-3 text-sm text-zinc-400">
                        "确认吊销当前设备的会话吗？吊销后本设备将立即退出登录, 且无法恢复。"
                    </p>

                    <div class="mt-6 flex gap-3">
                        <Button
                            variant="outline"
                            class="flex-1"
                            attr:data-testid="cancel-revoke-current"
                            attr:aria-label="取消吊销当前设备"
                            on:click=move |_| on_cancel.run(())
                        >
                            "取消"
                        </Button>
                        <Button
                            variant="destructive"
                            class="flex-1"
                            attr:data-testid="confirm-revoke-current"
                            attr:aria-label="确认吊销当前设备"
                            on:click=move |_| on_confirm.run(())
                        >
                            "确认吊销"
                        </Button>
                    </div>
                </div>
            </div>
        </Dialog>
    }
}

#[component]
pub fn SessionsPage() -> impl IntoView {
    let sessions = RwSignal::new(get_static_sessions());
    let busy = RwSignal::new(false);
    let confirm_sid = RwSignal::new(None::<String>);
    let notice = RwSignal::new(None::<String>);

    let search = RwSignal::new(String::new());
    let status_filter = RwSignal::new(SessionStatus::Active);

    // 当 confirm_sid 为 Some 时显示弹窗。
    // `Dialog::open` 要的是 `RwSignal<bool>`，用派生信号包一层再同步回 `confirm_sid`。
    let confirm_open = RwSignal::new(false);
    Effect::new(move |_| {
        confirm_open.set(confirm_sid.get().is_some());
    });

    let filtered_sessions = Memo::new(move |_| {
        let sessions_list = sessions.get();
        let search_term = search.get().to_lowercase();
        let status = status_filter.get();

        sessions_list
            .into_iter()
            .filter(|session| {
                if !search_term.is_empty() {
                    let ua_match = session.user_agent.to_lowercase().contains(&search_term);
                    let ip_match = session.ip.to_lowercase().contains(&search_term);
                    let method_match = session.login_method.to_lowercase().contains(&search_term);
                    if !(ua_match || ip_match || method_match) {
                        return false;
                    }
                }

                match status {
                    SessionStatus::Active => session.current,
                    SessionStatus::Inactive => !session.current,
                }
            })
            .collect::<Vec<_>>()
    });

    let on_revoke = Callback::new(move |sid: String| {
        if busy.get() {
            return;
        }
        busy.set(true);
        notice.set(None);

        set_timeout(
            move || {
                let mut items = sessions.get();
                items.retain(|s| s.sid != sid);
                sessions.set(items);
                notice.set(Some("会话已吊销".to_string()));
                busy.set(false);
            },
            std::time::Duration::from_millis(500),
        );
    });

    let on_confirm = Callback::new(move |_: ()| {
        if let Some(sid) = confirm_sid.get() {
            confirm_sid.set(None);
            if busy.get() {
                return;
            }
            busy.set(true);
            notice.set(None);

            set_timeout(
                move || {
                    let mut items = sessions.get();
                    items.retain(|s| s.sid != sid);
                    sessions.set(items);
                    notice.set(Some("当前设备会话已吊销".to_string()));
                    busy.set(false);
                },
                std::time::Duration::from_millis(800),
            );
        }
    });

    let on_cancel = Callback::new(move |_: ()| {
        confirm_sid.set(None);
    });

    let refresh_sessions = move |_| {
        sessions.set(get_static_sessions());
    };

    view! {
        <div class="flex flex-col gap-6">
            <div class="flex items-center justify-between gap-3">
                <div>
                    <h1 class="text-2xl font-bold text-zinc-100">"登录会话"</h1>
                    <p class="mt-1 text-sm text-zinc-500">"管理当前用户的所有设备会话"</p>
                </div>
                <Button
                    button_type="button"
                    variant="primary"
                    class="flex items-center gap-2"
                    on:click=move |_| refresh_sessions(())
                >
                    "刷新列表"
                </Button>
            </div>

            {move || notice.get().map(|msg| view! {
                <div class="rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2 text-sm text-zinc-300">
                    {msg}
                    {move || busy.get().then_some(" ···")}
                </div>
            })}

            <div class="flex flex-wrap items-center gap-3 rounded-xl border border-zinc-700/60 bg-zinc-900/60 p-4">
                <div class="flex-1 min-w-48">
                    <input
                        class="w-full rounded-lg border border-zinc-600 bg-zinc-800 px-3 py-2 text-sm text-zinc-100 placeholder-zinc-500 focus:border-indigo-500 focus:outline-none"
                        type="text"
                        placeholder="搜索用户代理、IP 或登录方式..."
                        prop:value=move || search.get()
                        on:input=move |ev| search.set(event_target_value(&ev))
                    />
                </div>

                <div class="flex gap-2">
                    <Button
                        button_type="button"
                        variant="primary"
                        class="px-4"
                        on:click=move |_| status_filter.set(SessionStatus::Active)
                    >
                        "活跃会话"
                    </Button>
                    <Button
                        button_type="button"
                        variant="outline"
                        class="px-4"
                        on:click=move |_| status_filter.set(SessionStatus::Inactive)
                    >
                        "非当前设备"
                    </Button>
                </div>
            </div>

            <div class="flex flex-col gap-4">
                {move || if filtered_sessions.get().is_empty() {
                    view! {
                        <div class="text-center py-12 text-zinc-500">
                            <p>"没有找到匹配的会话"</p>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <CardGrid>
                            {filtered_sessions.get().into_iter().map(|session| {
                                let sid = session.sid.clone();
                                view! {
                                    <SessionCard
                                        session=session
                                        busy=busy.get()
                                        on_revoke=Callback::new({
                                            let sid = sid.clone();
                                            let outer = on_revoke;
                                            move |_: String| outer.run(sid.clone())
                                        })
                                    />
                                }
                            }).collect_view()}
                        </CardGrid>
                    }.into_any()
                }}
            </div>

            <RevokeConfirmModal open=confirm_open on_confirm=on_confirm on_cancel=on_cancel/>
        </div>
    }
}
