use crate::ui::CardGrid;
use leptos::prelude::*;
use singlestage::*;

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
fn SessionCard(
    session: SessionData,
    busy: bool,
    on_revoke: impl FnMut(String) + 'static,
    on_request_current_revoke: impl FnMut(String) + 'static,
) -> impl IntoView {
    view! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-4",
            div { class: "flex items-start justify-between gap-3",
                div { class: "min-w-0",
                    div { class: "flex items-center gap-2",
                        p {
                            class: "truncate text-sm text-zinc-200",
                            title: "{session.user_agent}",
                            "{session.user_agent}"
                        }
                        if session.current {
                            span { class: "shrink-0 rounded-full bg-emerald-500/20 px-2 py-0.5 text-[10px] font-medium text-emerald-400", "当前设备" }
                        }
                    }
                    div { class: "mt-2 grid grid-cols-1 gap-1 text-xs text-zinc-500 sm:grid-cols-2",
                        span { "IP: {session.ip}" }
                        span { "登录方式: {session.login_method}" }
                        span { "最后活跃: {session.last_active}" }
                        span { "到期: {session.expires_at}" }
                    }
                }
                Button {
                    button_type: "button",
                    variant: if session.current { ButtonVariant::Destructive } else { ButtonVariant::Ghost },
                    class: "shrink-0 text-red-400",
                    disabled: busy,
                    "data-testid": "revoke-session-{session.sid}",
                    "aria-label": "吊销此会话",
                    onclick: move |_| {
                        if session.current {
                            on_request_current_revoke(session.sid.clone());
                        } else {
                            on_revoke(session.sid.clone());
                        }
                    },
                    "吊销"
                }
            }
        }
    }
}

#[component]
fn RevokeConfirmModal(
    open: RwSignal<bool>,
    sid: String,
    on_confirm: impl FnMut() + 'static,
    on_cancel: impl FnMut() + 'static,
) -> impl IntoView {
    view! {
        Dialog {
            open,
            on_cancel,
            div {
                class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
                role: "dialog",
                "aria-modal": "true",
                "aria-label": "吊销当前设备确认",
                "data-testid": "confirm-revoke-current-modal",
                onclick: move |_| on_cancel.call(()),
                div {
                    class: "w-full max-w-md rounded-2xl border border-red-500/40 bg-zinc-900 p-5 shadow-xl",
                    onclick: move |e| e.stop_propagation(),

                    h3 { class: "text-base font-semibold text-zinc-100", "吊销当前设备" }
                    p { class: "mt-3 text-sm text-zinc-400",
                        "确认吊销当前设备的会话吗？吊销后本设备将立即退出登录, 且无法恢复。"
                    }

                    div { class: "mt-6 flex gap-3",
                        Button {
                            variant: ButtonVariant::Outline,
                            class: "flex-1",
                            "data-testid": "cancel-revoke-current",
                            "aria-label": "取消吊销当前设备",
                            onclick: move |_| on_cancel.call(()),
                            "取消"
                        }
                        Button {
                            variant: ButtonVariant::Destructive,
                            class: "flex-1",
                            "data-testid": "confirm-revoke-current",
                            "aria-label": "确认吊销当前设备",
                            onclick: move |_| on_confirm.call(()),
                            "确认吊销"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SessionRow(
    session: SessionData,
    busy: bool,
    on_revoke: impl FnMut(String) + 'static,
    on_request_current_revoke: impl FnMut(String) + 'static,
) -> impl IntoView {
    SessionCard {
        session,
        busy,
        on_revoke,
        on_request_current_revoke,
    }
}

#[component]
pub fn SessionsPage() -> impl IntoView {
    let mut sessions = use_signal(|| get_static_sessions());
    let mut busy = use_signal(|| false);
    let mut confirm_sid = use_signal(|| None::<String>);
    let mut notice = use_signal(|| None::<String>);

    let mut search = use_signal(|| String::new());
    let mut status_filter = use_signal(|| SessionStatus::Active);

    // 当 confirm_sid 为 Some 时显示弹窗
    let confirm_open = Signal::derive(move || confirm_sid().is_some());

    let filtered_sessions = move || {
        let sessions_list = sessions();
        let search_term = search().to_lowercase();
        let status = status_filter();

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
    };

    let on_revoke_session = move |sid: String| {
        if busy() {
            return;
        }
        busy.set(true);
        notice.set(None);

        let mut sessions_sig = sessions;
        let mut busy_sig = busy;
        let mut notice_sig = notice;

        spawn_local(async move {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            let mut items = sessions_sig().to_vec();
            items.retain(|s| s.sid != sid);
            sessions_sig.set(items);
            notice_sig.set(Some("会话已吊销".to_string()));
            busy_sig.set(false);
        });
    };

    let on_request_current_revoke = move |sid: String| {
        confirm_sid.set(Some(sid));
    };

    let on_confirm_revoke = move || {
        if let Some(sid) = confirm_sid() {
            confirm_sid.set(None);
            if busy() {
                return;
            }
            busy.set(true);
            notice.set(None);

            let mut sessions_sig = sessions;
            let mut busy_sig = busy;
            let mut notice_sig = notice;

            spawn_local(async move {
                tokio::time::sleep(std::time::Duration::from_millis(800)).await;

                let mut items = sessions_sig().to_vec();
                items.retain(|s| s.sid != sid);
                sessions_sig.set(items);
                notice_sig.set(Some("当前设备会话已吊销".to_string()));
                busy_sig.set(false);
            });
        }
    };

    let on_cancel_revoke = move || {
        confirm_sid.set(None);
    };

    let refresh_sessions = move |_| {
        sessions.set(get_static_sessions());
    };

    view! {
        div { class: "flex flex-col gap-6",
            div { class: "flex items-center justify-between gap-3",
                div {
                    h1 { class: "text-2xl font-bold text-zinc-100", "登录会话" }
                    p { class: "mt-1 text-sm text-zinc-500", "管理当前用户的所有设备会话" }
                }
                Button {
                    button_type: "button",
                    variant: ButtonVariant::Primary,
                    class: "flex items-center gap-2",
                    onclick: move |_| refresh_sessions(()),
                    "刷新列表"
                }
            }

            if let Some(msg) = notice() {
                div {
                    class: "rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2 text-sm text-zinc-300",
                    "{msg}"
                    if busy() { " ···" }
                }
            }

            div { class: "flex flex-wrap items-center gap-3 rounded-xl border border-zinc-700/60 bg-zinc-900/60 p-4",
                div { class: "flex-1 min-w-48",
                    input {
                        class: "w-full rounded-lg border border-zinc-600 bg-zinc-800 px-3 py-2 text-sm text-zinc-100 placeholder-zinc-500 focus:border-indigo-500 focus:outline-none",
                        r#type: "text",
                        placeholder: "搜索用户代理、IP 或登录方式...",
                        value: search(),
                        oninput: move |ev| search.set(event_target_value(&ev)),
                    }
                }

                div { class: "flex gap-2",
                    Button {
                        button_type: "button",
                        variant: if matches!(status_filter(), SessionStatus::Active) { ButtonVariant::Primary } else { ButtonVariant::Outline },
                        class: "px-4",
                        onclick: move |_| status_filter.set(SessionStatus::Active),
                        "活跃会话"
                    }
                    Button {
                        button_type: "button",
                        variant: if matches!(status_filter(), SessionStatus::Inactive) { ButtonVariant::Primary } else { ButtonVariant::Outline },
                        class: "px-4",
                        onclick: move |_| status_filter.set(SessionStatus::Inactive),
                        "非当前设备"
                    }
                }
            }

            div { class: "flex flex-col gap-4",
                if filtered_sessions().is_empty() {
                    div { class: "text-center py-12 text-zinc-500",
                        p { "没有找到匹配的会话" }
                    }
                } else {
                    CardGrid {
                        for session in filtered_sessions() {
                            SessionRow {
                                session,
                                busy: busy(),
                                on_revoke: move |sid| on_revoke_session(sid),
                                on_request_current_revoke: move |sid| on_request_current_revoke(sid),
                            }
                        }
                    }
                }
            }

            RevokeConfirmModal {
                open: confirm_open,
                sid: confirm_sid().unwrap_or_default(),
                on_confirm: move || on_confirm_revoke(),
                on_cancel: move || on_cancel_revoke(),
            }
        }
    }
}
