//! 会话面板 — 列出当前用户全部存活会话（设备/登录方式/活跃与到期时间），
//! 并提供吊销操作: 行内吊销单条 (DELETE /api/user/self/sessions/{sid})、
//! 页头一键吊销其它设备会话 (POST /api/user/self/sessions/revoke-others)。
//! 「当前设备」的吊销有二次确认弹窗 (吊销后本设备立即退出登录);
//! 设备名由 `summarize_ua` 归纳为短标签, 时间经 `fmt_time_minute` 本地化。
//! 操作成功后刷新列表, 失败诚实展示后端错误。
//! 数据全部经 `api` 取用, 面板不认识数据怎么来。

use contract::api::user::SessionDto;
use dioxus::prelude::*;
use ui::button::{Button, ButtonSize, ButtonVariant};

use crate::api;
use crate::components::{ConfirmRevokeCurrentModal, SessionRow};

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
