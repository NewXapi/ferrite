//! 会话面板 — 列出当前用户全部存活会话（设备/登录方式/活跃与到期时间），
//! 并提供吊销操作: 行内吊销单条 (DELETE /api/user/self/sessions/{sid})、
//! 页头一键吊销其它设备会话 (POST /api/user/self/sessions/revoke-others)。
//! 操作成功后刷新列表, 失败诚实展示后端错误。
//! 数据全部经 `api` 取用, 面板不认识数据怎么来。

use contract::api::user::SessionDto;
use dioxus::prelude::*;
use ui::components::button::{Button, ButtonSize, ButtonVariant};

use crate::api;

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

#[component]
pub fn SessionsPanel() -> Element {
    let sessions = use_signal(|| None::<Vec<SessionDto>>);
    let err = use_signal(String::new);
    // 吊销操作状态: busy 防重入; flash 为成功提示 (保留到下次操作);
    // action_err 与加载 err 分开, 避免覆盖「无法加载」的上下文。
    let mut busy = use_signal(|| false);
    let mut flash = use_signal(|| None::<String>);
    let mut action_err = use_signal(String::new);

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
                                        match api::revoke_session_api(&client, &sid).await {
                                            Ok(_) => {
                                                f.set(Some("会话已吊销".into()));
                                                load_sessions(&client, s, e).await;
                                            }
                                            Err(e2) => ae.set(e2.to_string()),
                                        }
                                        b.set(false);
                                    });
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
        }
    }
}

/// 单条会话卡片。`on_revoke` 由父面板发起 DELETE 请求并刷新列表;
/// 吊销「当前设备」的会话会使本机登录态失效 (后端语义如此, 不做拦截)。
#[component]
fn SessionRow(session: SessionDto, busy: bool, on_revoke: EventHandler<String>) -> Element {
    // 精简 user_agent: 只取前 40 字符
    let ua_trunc: String = session.user_agent.chars().take(40).collect();
    let ua = if session.user_agent.len() > 40 {
        format!("{ua_trunc}…")
    } else {
        ua_trunc
    };

    rsx! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-4",
            div { class: "flex items-start justify-between gap-3",
                div { class: "min-w-0",
                    div { class: "flex items-center gap-2",
                        p { class: "truncate font-mono text-sm text-zinc-200", "{ua}" }
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
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Xs,
                    class: "shrink-0 text-red-400",
                    disabled: busy,
                    "data-testid": "revoke-session-{session.sid}",
                    "aria-label": "吊销此会话",
                    onclick: move |_| on_revoke.call(session.sid.clone()),
                    "吊销"
                }
            }
        }
    }
}
