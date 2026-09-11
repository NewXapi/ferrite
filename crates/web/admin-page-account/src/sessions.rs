//! 会话面板 — 只读列出当前用户全部存活会话（设备/登录方式/活跃与到期时间）。
//! 数据源: GET /api/user/self/sessions。会话仅作审计记录，不提供吊销操作。
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
            div {
                h2 { class: "text-lg font-medium text-zinc-100", "登录会话" }
                p { class: "mt-1 text-sm text-zinc-500", "当前用户全部存活设备与登录记录" }
            }

            if let Some(list) = sessions() {
                if list.is_empty() {
                    p { class: "text-sm text-zinc-500", "暂无存活会话" }
                } else {
                    div { class: "flex flex-col gap-3",
                        for s in list {
                            SessionRow { session: s }
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
fn SessionRow(session: SessionDto) -> Element {
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
            }
        }
    }
}
