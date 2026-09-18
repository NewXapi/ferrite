//! 出口代理节点导入面板 (M2-C):消费 `/api/proxy_nodes/subscription`
//! 与 `/api/proxy_nodes/batch`,呈现批量导入报告(已创建/已跳过/失败明细)。
//! 只做导入,运行时状态留给 `proxy_runtime`。

use dioxus::prelude::*;

use client::ApiClient;

/// 导入失败明细。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportFailure {
    pub source: String,
    pub reason: String,
}

/// 批量导入报告 (来自 /api/proxy_nodes/subscription 和 /batch)。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportReport {
    pub created: usize,
    pub skipped: usize,
    pub failures: Vec<ImportFailure>,
}

/// 导入请求 (camelCase 与后端一致)。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportRequest {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    pub channel_keys: Vec<String>,
    #[serde(default)]
    pub priority: i32,
}

/// 出口代理节点导入面板（只做导入，运行时状态留给 M3）。
/// 使用基本 dioxus 元素 (ponytail: 避免依赖未导出的 pub(crate) 组件)。
#[component]
pub fn ProxyNodesPanel() -> Element {
    let mut sub_url = use_signal(String::new);
    let mut share_text = use_signal(String::new);
    let mut channels_str = use_signal(String::new);
    let mut priority_str = use_signal(|| "10".to_string());
    let mut is_importing = use_signal(|| false);
    let mut report = use_signal(|| None::<ImportReport>);
    let mut err_msg = use_signal(|| None::<String>);

    let parse_channels = |s: &str| -> Vec<String> {
        s.split([',', '，'])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    let on_sub_import = move |_| {
        let url = sub_url();
        let ch = parse_channels(&channels_str());
        let pri = priority_str().parse::<i32>().unwrap_or(10);
        if url.trim().is_empty() || ch.is_empty() {
            err_msg.set(Some("URL 和渠道不能为空".to_string()));
            return;
        }

        let req = ImportRequest {
            url: Some(url),
            text: None,
            channel_keys: ch,
            priority: pri,
        };

        spawn(async move {
            is_importing.set(true);
            err_msg.set(None);
            report.set(None);

            let client = ApiClient::shared();
            match client.post("/api/proxy_nodes/subscription", &req).await {
                Ok(r) => report.set(Some(r)),
                Err(e) => err_msg.set(Some(e.to_string())),
            }
            is_importing.set(false);
        });
    };

    let on_share_import = move |_| {
        let text = share_text();
        let ch = parse_channels(&channels_str());
        let pri = priority_str().parse::<i32>().unwrap_or(10);
        if text.trim().is_empty() || ch.is_empty() {
            err_msg.set(Some("分享链接和渠道不能为空".to_string()));
            return;
        }

        let req = ImportRequest {
            url: None,
            text: Some(text),
            channel_keys: ch,
            priority: pri,
        };

        spawn(async move {
            is_importing.set(true);
            err_msg.set(None);
            report.set(None);

            let client = ApiClient::shared();
            match client.post("/api/proxy_nodes/batch", &req).await {
                Ok(r) => report.set(Some(r)),
                Err(e) => err_msg.set(Some(e.to_string())),
            }
            is_importing.set(false);
        });
    };

    rsx! {
        section {
            role: "region",
            "aria-label": "出口代理节点导入",
            id: "proxy-nodes-section",
            "data-testid": "proxy-nodes-panel",
            class: "scroll-mt-8 rounded-2xl border border-zinc-800 bg-zinc-900/60 p-6 space-y-6",

            div { class: "flex items-center justify-between mb-4",
                h2 { class: "text-lg font-semibold text-zinc-100", "出口代理节点" }
            }

            // 订阅导入
            div { class: "space-y-4 border-b border-zinc-800 pb-6",
                h3 { class: "text-sm font-medium text-zinc-300", "订阅导入" }
                div { class: "space-y-3",
                    input {
                        class: "w-full bg-zinc-950 border border-zinc-700 rounded-xl px-4 py-3 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-blue-500",
                        placeholder: "https://example.com/clash.yaml",
                        value: sub_url(),
                        oninput: move |e| sub_url.set(e.value()),
                        "data-testid": "proxy-subscription-url"
                    }
                    input {
                        class: "w-full bg-zinc-950 border border-zinc-700 rounded-xl px-4 py-3 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-blue-500",
                        placeholder: "openai,claude（逗号分隔）",
                        value: channels_str(),
                        oninput: move |e| channels_str.set(e.value()),
                    }
                    input {
                        class: "w-full bg-zinc-950 border border-zinc-700 rounded-xl px-4 py-3 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-blue-500",
                        placeholder: "优先级 (默认 10)",
                        value: priority_str(),
                        oninput: move |e| priority_str.set(e.value()),
                    }
                    button {
                        class: "w-full px-6 py-3 bg-blue-600 hover:bg-blue-500 disabled:bg-zinc-600 rounded-xl text-white text-sm font-medium transition-colors",
                        "data-testid": "proxy-subscription-import",
                        disabled: is_importing(),
                        onclick: on_sub_import,
                        if is_importing() { "导入中..." } else { "导入" }
                    }
                }
            }

            // 粘贴分享链接
            div { class: "space-y-4",
                h3 { class: "text-sm font-medium text-zinc-300", "粘贴分享链接" }
                textarea {
                    class: "w-full h-32 bg-zinc-950 border border-zinc-700 rounded-xl px-4 py-3 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-blue-500 font-mono resize-y",
                    placeholder: "vless://...\nvmess://...\nss://...",
                    value: share_text(),
                    oninput: move |e| share_text.set(e.value()),
                    "data-testid": "proxy-sharelink-text"
                }
                div { class: "grid grid-cols-2 gap-3",
                    input {
                        class: "bg-zinc-950 border border-zinc-700 rounded-xl px-4 py-3 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-blue-500",
                        placeholder: "openai,claude（逗号分隔）",
                        value: channels_str(),
                        oninput: move |e| channels_str.set(e.value()),
                    }
                    input {
                        class: "bg-zinc-950 border border-zinc-700 rounded-xl px-4 py-3 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-blue-500",
                        placeholder: "优先级 (默认 10)",
                        value: priority_str(),
                        oninput: move |e| priority_str.set(e.value()),
                    }
                }
                button {
                    class: "w-full px-6 py-3 bg-blue-600 hover:bg-blue-500 disabled:bg-zinc-600 rounded-xl text-white text-sm font-medium transition-colors",
                    "data-testid": "proxy-sharelink-import",
                    disabled: is_importing(),
                    onclick: on_share_import,
                    if is_importing() { "导入中..." } else { "导入" }
                }
            }

            // 结果展示
            if let Some(e) = err_msg() {
                div {
                    role: "alert",
                    class: "rounded-xl border border-red-500/30 bg-red-950/30 p-4 text-sm text-red-400",
                    "data-testid": "proxy-import-error",
                    "{e}"
                }
            } else if let Some(r) = report() {
                div {
                    role: "status",
                    class: "rounded-xl border border-emerald-500/30 bg-emerald-950/30 p-5",
                    "data-testid": "proxy-import-report",
                    div { class: "flex gap-6 text-sm mb-4",
                        span { class: "text-emerald-400 font-medium", "已创建: {r.created}" }
                        span { class: "text-amber-400 font-medium", "已跳过: {r.skipped}" }
                    }
                    if !r.failures.is_empty() {
                        div { class: "mt-3 pt-3 border-t border-zinc-700",
                            p { class: "text-xs text-zinc-400 mb-3", "失败明细（本功能核心卖点）：" }
                            for f in &r.failures {
                                div {
                                    class: "mb-2 text-xs p-3 bg-zinc-950 rounded border-l-4 border-red-500",
                                    "data-testid": "proxy-import-failure",
                                    span { class: "font-mono text-red-400", "{f.source}" }
                                    span { class: "text-zinc-500 mx-2", "→" }
                                    span { class: "text-zinc-300", "{f.reason}" }
                                }
                            }
                        }
                    } else {
                        p { class: "text-xs text-emerald-400/80", "所有节点导入成功" }
                    }
                }
            }
        }
    }
}
