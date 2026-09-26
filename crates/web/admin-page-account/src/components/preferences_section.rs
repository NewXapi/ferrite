//! 偏好设置区 — 读写自由 JSONB 设置 (GET/PUT /api/user/self/setting)。
//! 后端对设置做 deep-merge, 故前端只提交本次改动的键即可。

use dioxus::prelude::*;
use ui::button::{Button, ButtonVariant};

use crate::api;

/// 【是什么】偏好设置区段组件，读写账号的自由 JSONB 设置 (界面语言 / 接收通知) 并只读回显全量 JSON。
///
/// 【做什么】挂载时拉取全量设置并回填两个快捷字段；渲染语言下拉 + 通知开关 + 保存按钮 + 当前设置 JSON 只读视图；保存只提交本次改动的两个键 (后端 deep-merge)，成功后回填后端返回的最新 JSON。
///
/// 【交互逻辑】改下拉/勾选只改本地信号；点「保存设置」置 busy 防重入，成功显绿字 flash，失败在卡下方红字展示后端错误。
///
/// 【样式】标题 + 两栏网格 (sm 起两列)；输入与下拉 rounded-xl 边框 + focus:border-zinc-500；卡片 rounded-xl bg-zinc-900/60 p-6 + hover:border-zinc-600；JSON 视图 pre max-h-60 可滚动等宽小字。
///
/// 【子组件组成】ui::button::Button (保存设置) × 1
///
/// 【数据流】无 props：settings/err/flash/busy/language/notifications 全为本组件私有信号，经 api 取用；跨组件状态不外泄。
#[component]
pub fn PreferencesSection() -> Element {
    let settings = use_signal(|| None::<serde_json::Value>);
    let mut err = use_signal(String::new);
    let mut flash = use_signal(|| None::<String>);
    let busy = use_signal(|| false);

    // 快捷字段 (仅取常见键, 其余键保留在 JSON 里不受影响)
    let mut language = use_signal(String::new);
    let mut notifications = use_signal(|| false);

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
    });

    let raw_json = settings()
        .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
        .unwrap_or_else(|| "{}".into());

    rsx! {
        div { class: "flex flex-col gap-6",
            div {
                h2 { class: "{ui::TYPE_TITLE}", "偏好设置" }
                p { class: "mt-1 {ui::T_text_sm} {ui::T_text_zinc_500}", "设置以 JSONB 形式存于账号, 修改即时合并保存" }
            }

            div { class: "rounded-xl border {ui::T_border_zinc_800} bg-zinc-900/60 p-6 transition-colors hover:{ui::T_border_zinc_600}",
                div { class: "grid grid-cols-1 gap-4 sm:grid-cols-2",
                    div {
                        label { class: "mb-1.5 block {ui::T_text_xs} {ui::T_text_zinc_400}", "界面语言" }
                        select {
                            class: "{ui::INPUT}",
                            value: "{language}",
                            onchange: move |e| language.set(e.value()),
                            option { value: "zh", "简体中文" }
                            option { value: "en", "English" }
                            option { value: "", "跟随系统" }
                        }
                    }
                    div { class: "flex items-end",
                        label { class: "flex w-full cursor-pointer items-center justify-between rounded-xl border {ui::T_border_zinc_700} {ui::T_bg_zinc_950} px-4 py-2.5 {ui::T_text_sm}",
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
                        span { class: "{ui::T_text_xs} {ui::STATE_SUCCESS_TEXT}", "{m}" }
                    }
                }

                if !err().is_empty() {
                    p { class: "mt-3 {ui::T_text_xs} {ui::STATE_DANGER_TEXT}", "操作失败: {err()}" }
                }
            }

            div { class: "rounded-xl border {ui::T_border_zinc_800} bg-zinc-900/60 p-6 transition-colors hover:{ui::T_border_zinc_600}",
                div { class: "mb-3 flex items-center justify-between",
                    h3 { class: "{ui::TYPE_CARD_TITLE}", "当前设置 (JSON)" }
                    span { class: "{ui::TYPE_DESC}", "只读视图" }
                }
                pre { class: "max-h-60 overflow-auto rounded-lg {ui::T_bg_zinc_950} p-4 font-mono {ui::T_text_xs} {ui::T_text_zinc_300}", "{raw_json}" }
            }
        }
    }
}
