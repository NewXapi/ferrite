//! 用户设置面板 — 读写自由 JSONB 设置 (GET/PUT /api/user/self/setting)。
//! 后端对设置做 deep-merge, 故前端只提交本次改动的键即可。
//! 数据全部经 `api` 取用, 面板不认识数据怎么来。

use dioxus::prelude::*;

use crate::api;

#[component]
pub fn SettingsPanel() -> Element {
    let mut settings = use_signal(|| None::<serde_json::Value>);
    let mut err = use_signal(String::new);
    let mut flash = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);

    // 快捷字段 (仅取常见键, 其余键保留在 JSON 里不受影响)
    let mut language = use_signal(String::new);
    let mut notifications = use_signal(|| false);

    use_hook(move || {
        let client = client::ApiClient::shared().clone();
        let mut s = settings.clone();
        let mut e = err.clone();
        let mut lang = language.clone();
        let mut notif = notifications.clone();
        spawn(async move {
            match api::get_settings_api(&client).await {
                Ok(v) => {
                    s.set(Some(v.clone()));
                    if let Some(l) = v.get("language").and_then(|x| x.as_str()) {
                        lang.set(l.to_string());
                    }
                    if let Some(n) = v.get("notifications").and_then(|x| x.as_bool()) {
                        notif.set(n);
                    }
                }
                Err(e2) => e.set(e2.to_string()),
            }
        });
    });

    let raw_json = settings()
        .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
        .unwrap_or_else(|| "{}".into());

    rsx! {
        div { class: "flex flex-col gap-6",
            div {
                h2 { class: "text-lg font-medium text-zinc-100", "偏好设置" }
                p { class: "mt-1 text-sm text-zinc-500", "设置以 JSONB 形式存于账号, 修改即时合并保存" }
            }

            div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-6",
                div { class: "grid grid-cols-1 gap-4 sm:grid-cols-2",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "界面语言" }
                        select {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm focus:border-zinc-500 focus:outline-none",
                            value: "{language}",
                            onchange: move |e| language.set(e.value()),
                            option { value: "zh", "简体中文" }
                            option { value: "en", "English" }
                            option { value: "", "跟随系统" }
                        }
                    }
                    div { class: "flex items-end",
                        label { class: "flex w-full cursor-pointer items-center justify-between rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm",
                            span { "接收通知" }
                            input {
                                r#type: "checkbox",
                                class: "h-4 w-4 accent-emerald-500",
                                checked: "{notifications()}",
                                onchange: move |e| notifications.set(e.checked()),
                            }
                        }
                    }
                }

                div { class: "mt-4 flex items-center gap-3",
                    button {
                        class: "rounded-xl bg-white px-4 py-2 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200 disabled:opacity-40",
                        disabled: busy(),
                        onclick: move |_| {
                            if busy() {
                                return;
                            }
                            let mut b = busy.clone();
                            b.set(true);
                            flash.set(None);
                            err.set(String::new());
                            let client = client::ApiClient::shared().clone();
                            let mut s = settings.clone();
                            let mut f = flash.clone();
                            let mut e = err.clone();
                            let payload = serde_json::json!({
                                "language": language(),
                                "notifications": notifications(),
                            });
                            spawn(async move {
                                match api::update_settings_api(&client, &payload).await {
                                    Ok(v) => {
                                        s.set(Some(v));
                                        f.set(Some("设置已保存".into()));
                                    }
                                    Err(e2) => e.set(e2.to_string()),
                                }
                                b.set(false);
                            });
                        },
                        "保存设置"
                    }
                    if let Some(m) = flash() {
                        span { class: "text-xs text-emerald-400", "{m}" }
                    }
                }

                if !err().is_empty() {
                    p { class: "mt-3 text-xs text-red-400", "操作失败: {err()}" }
                }
            }

            div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-6",
                div { class: "mb-3 flex items-center justify-between",
                    h3 { class: "text-sm font-medium text-zinc-100", "当前设置 (JSON)" }
                    span { class: "text-xs text-zinc-500", "只读视图" }
                }
                pre { class: "max-h-60 overflow-auto rounded-lg bg-zinc-950 p-4 font-mono text-xs text-zinc-300", "{raw_json}" }
            }
        }
    }
}
