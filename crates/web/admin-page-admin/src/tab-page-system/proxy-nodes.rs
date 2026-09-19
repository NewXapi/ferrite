//! 出口代理节点导入面板 (M2-C):消费 `/api/proxy_nodes/subscription`
//! 与 `/api/proxy_nodes/batch`,呈现批量导入报告(已创建/已跳过/失败明细)。
//! 只做导入,运行时状态留给 `proxy_runtime`。

use dioxus::prelude::*;

use super::shared::{
    BTN_IMPORT, BTN_IMPORTING, FIELD_CHANNELS_PLACEHOLDER, FIELD_PRIORITY_PLACEHOLDER,
    FIELD_SHARE_IMPORT, FIELD_SUB_IMPORT, FIELD_SUB_URL_PLACEHOLDER, LBL_PROXY_NODES,
    MSG_ALL_IMPORTED, MSG_CREATED_PREFIX, MSG_FAILURES_TITLE, MSG_IMPORT_SHARE_REQUIRED,
    MSG_IMPORT_URL_REQUIRED, MSG_SKIPPED_PREFIX, SEC_PROXY_IMPORT,
};
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
///
/// 【是什么】出口代理节点导入面板:订阅链接导入 + 粘贴分享链接导入两条通路,
/// 加一个导入结果报告区。
///
/// 【做什么】把用户填的订阅 URL / 分享链接文本 + 渠道列表 + 优先级组装成
/// `ImportRequest`,分别 POST `/api/proxy_nodes/subscription` 与
/// `/api/proxy_nodes/batch`,渲染已创建/已跳过/失败明细三类反馈。
/// 不负责节点运行时状态(在 `proxy_runtime`),不负责渠道本身的 CRUD。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 输入 URL / 分享文本 / 渠道 / 优先级 → 写回组件内 `sub_url` / `share_text`
///   / `channels_str` / `priority_str`(纯本地 signal,不发网络)。
/// - 点「导入」→ 先做必填校验(URL/分享链接与渠道均非空),不过则把提示写入
///   `err_msg` 并提前返回;通过则 `spawn` 异步 POST,期间 `is_importing` 置真
///   禁用按钮,结束后按 Ok/Err 写 `report` / `err_msg`。
/// 数据交互:两条导入路径各触发一次 POST;无其它网络调用。
///
/// 【样式】外壳 `section#proxy-nodes-section` 为 `scroll-mt-8 rounded-2xl
/// border border-zinc-800 bg-zinc-900/60 p-6 space-y-6`;两条通路用
/// `border-b border-zinc-800 pb-6` 分隔,标题 h3 `text-sm font-medium
/// text-zinc-300`;输入框统一 `bg-zinc-950 border border-zinc-700 rounded-xl`
/// + `placeholder-zinc-500 focus:border-blue-500`;导入按钮 `bg-blue-600
/// hover:bg-blue-500 disabled:bg-zinc-600`;结果区错误红底 / 报告绿底
/// (`border-emerald-500/30 bg-emerald-950/30`),失败明细行左红边框。
///
/// 【子组件组成】无子组件:全部为原生 dioxus 元素(`section` / `div` / `h2`
/// / `h3` / `input` / `textarea` / `button` / `span` / `p`)。
///
/// 【数据流】
/// - 对内(入):无 props;`sub_url` / `share_text` / `channels_str` /
///   `priority_str` / `is_importing` / `report` / `err_msg` 七个 signal 均为
///   组件内 `use_signal`,不跨组件共享。
/// - 对外(出):无 EventHandler / 无 signal 写回;两个导入闭包只改本组件
///   signal,导入成功后不发 `bump_topo_refresh`(与拓扑画布无联动)。
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
            err_msg.set(Some(MSG_IMPORT_URL_REQUIRED.to_string()));
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
            err_msg.set(Some(MSG_IMPORT_SHARE_REQUIRED.to_string()));
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
            "aria-label": SEC_PROXY_IMPORT,
            id: "proxy-nodes-section",
            "data-testid": "proxy-nodes-panel",
            class: "scroll-mt-8 rounded-2xl border border-zinc-800 bg-zinc-900/60 p-6 space-y-6",

            div { class: "flex items-center justify-between mb-4",
                h2 { class: "text-lg font-semibold text-zinc-100", {LBL_PROXY_NODES} }
            }

            // 订阅导入
            div { class: "space-y-4 border-b border-zinc-800 pb-6",
                h3 { class: "text-sm font-medium text-zinc-300", {FIELD_SUB_IMPORT} }
                div { class: "space-y-3",
                    input {
                        class: "w-full bg-zinc-950 border border-zinc-700 rounded-xl px-4 py-3 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-blue-500",
                        placeholder: FIELD_SUB_URL_PLACEHOLDER,
                        value: sub_url(),
                        oninput: move |e| sub_url.set(e.value()),
                        "data-testid": "proxy-subscription-url"
                    }
                    input {
                        class: "w-full bg-zinc-950 border border-zinc-700 rounded-xl px-4 py-3 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-blue-500",
                        placeholder: FIELD_CHANNELS_PLACEHOLDER,
                        value: channels_str(),
                        oninput: move |e| channels_str.set(e.value()),
                    }
                    input {
                        class: "w-full bg-zinc-950 border border-zinc-700 rounded-xl px-4 py-3 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-blue-500",
                        placeholder: FIELD_PRIORITY_PLACEHOLDER,
                        value: priority_str(),
                        oninput: move |e| priority_str.set(e.value()),
                    }
                    button {
                        class: "w-full px-6 py-3 bg-blue-600 hover:bg-blue-500 disabled:bg-zinc-600 rounded-xl text-white text-sm font-medium transition-colors",
                        "data-testid": "proxy-subscription-import",
                        disabled: is_importing(),
                        onclick: on_sub_import,
                        if is_importing() { {BTN_IMPORTING} } else { {BTN_IMPORT} }
                    }
                }
            }

            // 粘贴分享链接
            div { class: "space-y-4",
                h3 { class: "text-sm font-medium text-zinc-300", {FIELD_SHARE_IMPORT} }
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
                        placeholder: FIELD_CHANNELS_PLACEHOLDER,
                        value: channels_str(),
                        oninput: move |e| channels_str.set(e.value()),
                    }
                    input {
                        class: "bg-zinc-950 border border-zinc-700 rounded-xl px-4 py-3 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:border-blue-500",
                        placeholder: FIELD_PRIORITY_PLACEHOLDER,
                        value: priority_str(),
                        oninput: move |e| priority_str.set(e.value()),
                    }
                }
                button {
                    class: "w-full px-6 py-3 bg-blue-600 hover:bg-blue-500 disabled:bg-zinc-600 rounded-xl text-white text-sm font-medium transition-colors",
                    "data-testid": "proxy-sharelink-import",
                    disabled: is_importing(),
                    onclick: on_share_import,
                    if is_importing() { {BTN_IMPORTING} } else { {BTN_IMPORT} }
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
                        span { class: "text-emerald-400 font-medium", {MSG_CREATED_PREFIX} "{r.created}" }
                        span { class: "text-amber-400 font-medium", {MSG_SKIPPED_PREFIX} "{r.skipped}" }
                    }
                    if !r.failures.is_empty() {
                        div { class: "mt-3 pt-3 border-t border-zinc-700",
                            p { class: "text-xs text-zinc-400 mb-3", {MSG_FAILURES_TITLE} }
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
                        p { class: "text-xs text-emerald-400/80", {MSG_ALL_IMPORTED} }
                    }
                }
            }
        }
    }
}
