//! 会话面板 — 列出当前用户全部存活会话（设备/登录方式/活跃与到期时间），
//! 并提供吊销操作: 行内吊销单条 (DELETE /api/user/self/sessions/{sid})、
//! 页头一键吊销其它设备会话 (POST /api/user/self/sessions/revoke-others)。
//! 「当前设备」的吊销有二次确认弹窗 (吊销后本设备立即退出登录);
//! 设备名由 `summarize_ua` 归纳为短标签, 时间经 `fmt_time_minute` 本地化。
//! 操作成功后刷新列表, 失败诚实展示后端错误。
//! 数据全部经 `api` 取用, 面板不认识数据怎么来。

use contract::api::user::SessionDto;
use dioxus::prelude::*;
use ui::components::button::{Button, ButtonSize, ButtonVariant};

use crate::api;
use crate::usage_support::{fmt_time_minute, summarize_ua};

/// 拉取会话列表, 结果写入 sessions / err 信号。信号按值 (clone 句柄) 接收并 `mut`。
/// 首次加载与吊销成功后的刷新共用此入口。
async fn load_sessions(
    client: &client::ApiClient,
    mut sessions: Signal<Option<Vec<SessionDto>>>,
    mut err: Signal<String>,
) {
    match api::list_sessions_api(client).await {
        Ok(v) => sessions.set(Some(v)),
        Err(e) => err.set(e.to_string()),
    }
}

/// 吊销单条会话并刷新列表 (行内直吊与确认弹窗共用此流程)。
/// busy 由调用方前置检查 (这里置位并复位); 成功/失败分别写 flash / action_err。
async fn revoke_session_flow(
    client: &client::ApiClient,
    sid: &str,
    sessions: Signal<Option<Vec<SessionDto>>>,
    err: Signal<String>,
    mut flash: Signal<Option<String>>,
    mut action_err: Signal<String>,
    mut busy: Signal<bool>,
) {
    busy.set(true);
    flash.set(None);
    action_err.set(String::new());
    match api::revoke_session_api(client, sid).await {
        Ok(_) => {
            flash.set(Some("会话已吊销".into()));
            load_sessions(client, sessions, err).await;
        }
        Err(e2) => action_err.set(e2.to_string()),
    }
    busy.set(false);
}

#[component]
pub fn SessionsPanel() -> Element {
    let sessions = use_signal(|| None::<Vec<SessionDto>>);
    let err = use_signal(String::new);
    // 吊销操作状态: busy 防重入; flash 为成功提示 (保留到下次操作);
    // action_err 与加载 err 分开, 避免覆盖「无法加载」的上下文。
    let mut busy = use_signal(|| false);
    let mut flash = use_signal(|| None::<String>);
    let mut action_err = use_signal(String::new);
    // 当前设备吊销确认: Some(sid) = 待确认的会话 (弹窗展示), 确认后才真正调 DELETE。
    let mut confirm_current = use_signal(|| None::<String>);

    use_hook(move || {
        let client = client::ApiClient::shared().clone();
        let s = sessions;
        let e = err;
        spawn(async move {
            load_sessions(&client, s, e).await;
        });
    });

    rsx! {
        div { class: "flex flex-col gap-4", role: "region", "aria-label": "登录会话面板", "data-testid": "sessions-panel",
            div { class: "flex items-center justify-between gap-3",
                div {
                    h2 { class: "text-lg font-medium text-zinc-100", "登录会话" }
                    p { class: "mt-1 text-sm text-zinc-500", "当前用户全部存活设备与登录记录" }
                }
                Button {
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    disabled: busy(),
                    "data-testid": "revoke-others-sessions",
                    "aria-label": "吊销其他会话",
                    onclick: move |_| {
                        if busy() {
                            return;
                        }
                        busy.set(true);
                        flash.set(None);
                        action_err.set(String::new());
                        let client = client::ApiClient::shared().clone();
                        let s = sessions;
                        let e = err;
                        let mut f = flash;
                        let mut ae = action_err;
                        let mut b = busy;
                        spawn(async move {
                            match api::revoke_others_sessions_api(&client).await {
                                Ok(_) => {
                                    f.set(Some("其它设备会话已全部吊销".into()));
                                    load_sessions(&client, s, e).await;
                                }
                                Err(e2) => ae.set(e2.to_string()),
                            }
                            b.set(false);
                        });
                    },
                    "吊销其他会话"
                }
            }

            if let Some(m) = flash() {
                p { class: "text-xs text-emerald-400", "{m}" }
            }
            if !action_err().is_empty() {
                p { class: "text-xs text-red-400", "吊销失败: {action_err()}" }
            }

            if let Some(list) = sessions() {
                if list.is_empty() {
                    p { class: "text-sm text-zinc-500", "暂无存活会话" }
                } else {
                    div { class: "flex flex-col gap-3",
                        for s in list {
                            SessionRow {
                                session: s,
                                busy: busy(),
                                on_revoke: move |sid: String| {
                                    if busy() {
                                        return;
                                    }
                                    let client = client::ApiClient::shared().clone();
                                    spawn(async move {
                                        revoke_session_flow(
                                            &client,
                                            &sid,
                                            sessions,
                                            err,
                                            flash,
                                            action_err,
                                            busy,
                                        )
                                        .await;
                                    });
                                },
                                on_request_current_revoke: move |sid: String| {
                                    // 当前设备: 只弹确认, 不直接吊销 (吊销即本机登出)。
                                    confirm_current.set(Some(sid));
                                },
                            }
                        }
                    }
                }
            } else if !err().is_empty() {
                p { class: "text-sm text-amber-400", "无法加载会话 (未登录或请求失败): {err()}" }
            } else {
                p { class: "text-sm text-zinc-500", "加载中…" }
            }

            // 当前设备吊销确认弹窗 (fixed 覆盖层, 位置无关渲染)
            if let Some(sid) = confirm_current() {
                ConfirmRevokeCurrentModal {
                    on_cancel: move |_| confirm_current.set(None),
                    on_confirmed: move |_| {
                        confirm_current.set(None);
                        if busy() {
                            return;
                        }
                        // EventHandler 是可多次调用的 Fn, sid 进 async 前先克隆
                        let sid = sid.clone();
                        let client = client::ApiClient::shared().clone();
                        spawn(async move {
                            revoke_session_flow(
                                &client,
                                &sid,
                                sessions,
                                err,
                                flash,
                                action_err,
                                busy,
                            )
                            .await;
                        });
                    },
                }
            }
        }
    }
}

/// 单条会话卡片。`on_revoke` 由父面板发起 DELETE 请求并刷新列表。
/// 「当前设备」走 `on_request_current_revoke` 先弹确认 (吊销即本机立即登出),
/// 其他设备维持行内直接吊销。
#[component]
fn SessionRow(
    session: SessionDto,
    busy: bool,
    on_revoke: EventHandler<String>,
    on_request_current_revoke: EventHandler<String>,
) -> Element {
    // UA 归纳为「浏览器 · OS」短标签, 完整 UA 挂 title 悬停可见 (不截断丢信息)。
    let ua_label = summarize_ua(&session.user_agent);
    // 时间展示: 本地时区分钟精度 (含年份); 解析失败时 fmt_time_minute 原样返回,
    // 保证后端异常数据仍诚实可见而不是变成空串。
    let last_active = fmt_time_minute(&session.last_active);
    let expires_at = fmt_time_minute(&session.expires_at);

    rsx! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-4",
            div { class: "flex items-start justify-between gap-3",
                div { class: "min-w-0",
                    div { class: "flex items-center gap-2",
                        p {
                            class: "truncate text-sm text-zinc-200",
                            title: "{session.user_agent}",
                            "{ua_label}"
                        }
                        if session.current {
                            span { class: "shrink-0 rounded-full bg-emerald-500/20 px-2 py-0.5 text-[10px] font-medium text-emerald-400", "当前设备" }
                        }
                    }
                    div { class: "mt-2 grid grid-cols-1 gap-1 text-xs text-zinc-500 sm:grid-cols-2",
                        span { "IP: {session.ip}" }
                        span { "登录方式: {session.login_method}" }
                        // 时间悬停可见原始 RFC3339 (与上方 UA 短标签同款处理)
                        span { title: "{session.last_active}", "最后活跃: {last_active}" }
                        span { title: "{session.expires_at}", "到期: {expires_at}" }
                    }
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Xs,
                    class: "shrink-0 text-red-400",
                    disabled: busy,
                    "data-testid": "revoke-session-{session.sid}",
                    "aria-label": "吊销此会话",
                    onclick: move |_| {
                        if session.current {
                            on_request_current_revoke.call(session.sid.clone());
                        } else {
                            on_revoke.call(session.sid.clone());
                        }
                    },
                    "吊销"
                }
            }
        }
    }
}

/// 吊销「当前设备」确认弹窗 — 视觉仿 keys.rs 的删除确认弹窗 (DeleteKeyModal)。
/// 只负责确认交互, 真正的 DELETE 由父面板在 on_confirmed 里执行;
/// 「吊销其他设备」不弹窗, 维持现状。
#[component]
fn ConfirmRevokeCurrentModal(
    on_cancel: EventHandler<()>,
    on_confirmed: EventHandler<()>,
) -> Element {
    rsx! {
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
                        onclick: move |_| on_confirmed.call(()),
                        "确认吊销"
                    }
                }
            }
        }
    }
}
