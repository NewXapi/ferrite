//! 会话面板 — 列出当前用户全部存活会话, 支持吊销指定会话 / 一键吊销其它设备。
//! 数据源: GET /api/user/self/sessions, DELETE .../sessions/{sid}, POST .../sessions/revoke-others。
//! 数据全部经 `api` 取用, 面板不认识数据怎么来。

use contract::api::user::SessionDto;
use dioxus::prelude::*;

use crate::api;

/// 拉取会话列表, 结果写入 sessions / err 信号。信号按值 (clone 句柄) 接收并 `mut`。
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
    let flash = use_signal(|| None::<String>);

    use_hook(move || {
        let client = client::ApiClient::shared().clone();
        let s = sessions;
        let e = err;
        spawn(async move {
            load_sessions(&client, s, e).await;
        });
    });

    rsx! {
        div { class: "flex flex-col gap-4",
            div { class: "flex items-center justify-between",
                div {
                    h2 { class: "text-lg font-medium text-zinc-100", "登录会话" }
                    p { class: "mt-1 text-sm text-zinc-500", "当前用户全部存活设备, 可随时吊销其它会话" }
                }
                button {
                    class: "rounded-lg border border-zinc-700 bg-zinc-800/60 px-3 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white",
                    onclick: move |_| {
                        let client = client::ApiClient::shared().clone();
                        let s = sessions;
                        let e = err;
                        let mut f = flash;
                        spawn(async move {
                            match api::revoke_others_sessions_api(&client).await {
                                Ok(_) => {
                                    f.set(Some("已吊销其它设备会话".into()));
                                    load_sessions(&client, s, e).await;
                                }
                                Err(e2) => {
                                    f.set(Some(format!("操作失败: {e2}")));
                                }
                            }
                        });
                    },
                    "吊销其它设备"
                }
            }

            if let Some(m) = flash() {
                p { class: "text-xs text-emerald-400", "{m}" }
            }

            if let Some(list) = sessions() {
                if list.is_empty() {
                    p { class: "text-sm text-zinc-500", "暂无存活会话" }
                } else {
                    div { class: "flex flex-col gap-3",
                        for s in list {
                            SessionRow {
                                session: s,
                                on_revoke: move |sid: String| {
                                    let client = client::ApiClient::shared().clone();
                                    let sk = sessions;
                                    let ek = err;
                                    let mut fk = flash;
                                    spawn(async move {
                                        match api::revoke_session_api(&client, &sid).await {
                                            Ok(_) => {
                                                fk.set(Some("已吊销该会话".into()));
                                                load_sessions(&client, sk, ek).await;
                                            }
                                            Err(e2) => {
                                                fk.set(Some(format!("吊销失败: {e2}")));
                                            }
                                        }
                                    });
                                }
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

#[component]
fn SessionRow(session: SessionDto, on_revoke: EventHandler<String>) -> Element {
    // 精简 user_agent: 只取前 40 字符
    let ua_trunc: String = session.user_agent.chars().take(40).collect();
    let ua = if session.user_agent.len() > 40 {
        format!("{ua_trunc}…")
    } else {
        ua_trunc
    };
    let sid = session.sid.clone();

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
                if !session.current {
                    button {
                        class: "shrink-0 rounded-lg border border-red-500/40 bg-red-500/10 px-2.5 py-1 text-xs font-medium text-red-400 transition-colors hover:bg-red-500/20 hover:text-red-300",
                        onclick: move |_| on_revoke.call(sid.clone()),
                        "吊销"
                    }
                }
            }
        }
    }
}
