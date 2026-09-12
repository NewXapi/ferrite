//! 用户设置面板 — 读写自由 JSONB 设置 (GET/PUT /api/user/self/setting)。
//! 后端对设置做 deep-merge, 故前端只提交本次改动的键即可。
//! 另含「资料与密码」编辑区: PUT /api/user/self 改显示名 / 改密码
//! (改密须原密码 + 新密码成对提供)。
//! 数据全部经 `api` 取用, 面板不认识数据怎么来。

use contract::api::user::UpdateSelfRequest;
use dioxus::prelude::*;
use ui::components::button::{Button, ButtonVariant};

use crate::api;

#[component]
pub fn SettingsPanel() -> Element {
    let settings = use_signal(|| None::<serde_json::Value>);
    let mut err = use_signal(String::new);
    let mut flash = use_signal(|| None::<String>);
    let busy = use_signal(|| false);

    // 快捷字段 (仅取常见键, 其余键保留在 JSON 里不受影响)
    let mut language = use_signal(String::new);
    let mut notifications = use_signal(|| false);

    // —— 资料与密码 (PUT /api/user/self) ——
    let mut display_name = use_signal(String::new);
    let mut original_password = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    // 保存动作状态 (显示名 / 密码共用: 同一端点, 串行防重入)
    let mut saving = use_signal(|| false);
    let mut save_err = use_signal(String::new);
    let mut save_flash = use_signal(|| None::<String>);

    use_hook(move || {
        let client = client::ApiClient::shared().clone();
        let mut s = settings;
        let mut e = err;
        let mut lang = language;
        let mut notif = notifications;
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

        // 预填当前显示名 (GET /api/user/self)
        let client = client::ApiClient::shared().clone();
        let mut dn = display_name;
        spawn(async move {
            if let Ok(u) = api::get_self_api(&client).await {
                dn.set(u.display_name);
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
                    Button {
                        variant: ButtonVariant::Primary,
                        disabled: busy(),
                        onclick: move |_| {
                            if busy() {
                                return;
                            }
                            let mut b = busy;
                            b.set(true);
                            flash.set(None);
                            err.set(String::new());
                            let client = client::ApiClient::shared().clone();
                            let mut s = settings;
                            let mut f = flash;
                            let mut e = err;
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

            // —— 资料与密码: PUT /api/user/self (display_name / original_password + new_password) ——
            div {
                class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-6",
                role: "group",
                "aria-label": "资料与密码",
                "data-testid": "settings-account-card",
                div { class: "mb-3 flex items-center justify-between",
                    h3 { class: "text-sm font-medium text-zinc-100", "资料与密码" }
                    span { class: "text-xs text-zinc-500", "改密须原密码 + 新密码成对提供" }
                }

                div { class: "grid grid-cols-1 gap-4 sm:grid-cols-2",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "显示名" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm focus:border-zinc-500 focus:outline-none",
                            "data-testid": "settings-display-name",
                            placeholder: "对外展示的名称",
                            value: "{display_name}",
                            oninput: move |e| display_name.set(e.value()),
                        }
                    }
                    div { class: "flex items-end",
                        Button {
                            variant: ButtonVariant::Primary,
                            disabled: saving(),
                            "data-testid": "save-display-name",
                            "aria-label": "保存显示名",
                            onclick: move |_| {
                                if saving() {
                                    return;
                                }
                                let name = display_name().trim().to_string();
                                if name.is_empty() {
                                    save_err.set("显示名不能为空".into());
                                    save_flash.set(None);
                                    return;
                                }
                                saving.set(true);
                                save_err.set(String::new());
                                save_flash.set(None);
                                let client = client::ApiClient::shared().clone();
                                let req = UpdateSelfRequest {
                                    display_name: Some(name),
                                    original_password: None,
                                    new_password: None,
                                };
                                let mut f = save_flash;
                                let mut e2 = save_err;
                                let mut sv = saving;
                                spawn(async move {
                                    match api::update_self_api(&client, &req).await {
                                        Ok(u) => {
                                            // 同步共享缓存, 顶栏 UserBadge 立即读到新显示名
                                            if let Ok(s) = serde_json::to_string(&u) {
                                                ui::set_storage_item("ferrite_current_user", &s);
                                            }
                                            f.set(Some("显示名已更新".into()));
                                        }
                                        Err(err2) => e2.set(err2.to_string()),
                                    }
                                    sv.set(false);
                                });
                            },
                            "保存显示名"
                        }
                    }
                }

                div { class: "mt-4 grid grid-cols-1 gap-4 sm:grid-cols-2",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "原密码" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 font-mono text-sm focus:border-zinc-500 focus:outline-none",
                            r#type: "password",
                            "data-testid": "settings-original-password",
                            placeholder: "当前密码",
                            value: "{original_password}",
                            oninput: move |e| original_password.set(e.value()),
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "新密码" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 font-mono text-sm focus:border-zinc-500 focus:outline-none",
                            r#type: "password",
                            "data-testid": "settings-new-password",
                            placeholder: "留空 = 不修改密码",
                            value: "{new_password}",
                            oninput: move |e| new_password.set(e.value()),
                        }
                    }
                }

                div { class: "mt-4 flex items-center gap-3",
                    Button {
                        variant: ButtonVariant::Primary,
                        disabled: saving(),
                        "data-testid": "save-password",
                        "aria-label": "修改密码",
                        onclick: move |_| {
                            if saving() {
                                return;
                            }
                            let orig = original_password();
                            let next = new_password();
                            // 后端语义: 改密必须原密码 + 新密码成对; 两者都空 = 不改密, 不发请求
                            if orig.is_empty() && next.is_empty() {
                                return;
                            }
                            if orig.is_empty() || next.is_empty() {
                                save_err.set("修改密码须同时填写原密码与新密码".into());
                                save_flash.set(None);
                                return;
                            }
                            saving.set(true);
                            save_err.set(String::new());
                            save_flash.set(None);
                            let client = client::ApiClient::shared().clone();
                            let req = UpdateSelfRequest {
                                display_name: None,
                                original_password: Some(orig),
                                new_password: Some(next),
                            };
                            let mut op = original_password;
                            let mut np = new_password;
                            let mut f = save_flash;
                            let mut e2 = save_err;
                            let mut sv = saving;
                            spawn(async move {
                                match api::update_self_api(&client, &req).await {
                                    Ok(_) => {
                                        op.set(String::new());
                                        np.set(String::new());
                                        f.set(Some("密码已修改".into()));
                                    }
                                    Err(err2) => e2.set(err2.to_string()),
                                }
                                sv.set(false);
                            });
                        },
                        "修改密码"
                    }
                    if let Some(m) = save_flash() {
                        span { class: "text-xs text-emerald-400", "{m}" }
                    }
                }

                if !save_err().is_empty() {
                    p { class: "mt-3 text-xs text-red-400", "操作失败: {save_err()}" }
                }
            }
        }
    }
}
