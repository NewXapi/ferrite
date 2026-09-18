//! 代理节点运行态面板 (M3):消费 `GET /api/proxy_nodes/report`,把网关数据面
//! (ProxyManager)每节点的 inflight/失败计数/冷却剩余/最近延迟 join DB 行
//! 呈现出来。三态诚实:loading 占位、错误透出后端信息、空态明示无代理节点;
//! stats 缺失的行(停用/未装配)明示"无运行态",不拿全 0 冒充。

use dioxus::prelude::*;

use client::ApiClient;

/// 节点运行态指标,对应 `GET /api/proxy_nodes/report` 每行的 `stats` 字段,
/// 即后端 `gateway_proxy::manager::NodeStats` 的 JSON 投影。
///
/// `last_delay_ms` 为 `None`(JSON null)表示从未探测/未装配,不是 0 延迟;
/// `cooldown_remaining_secs` 为 0 表示未进入冷却(后端语义)。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeRuntimeStats {
    #[serde(default)]
    pub inflight: u32,
    #[serde(default)]
    pub failure_count: u32,
    #[serde(default)]
    pub cooldown_remaining_secs: u64,
    #[serde(default)]
    pub last_delay_ms: Option<u16>,
}

/// report 单行:后端 `admin_proxy::ProxyNodeView`(camelCase)join 运行态 `stats`。
///
/// `stats` 为 `None` 表示该行没有运行时身份——节点已停用(disabled),或未被
/// 数据面快照装配(如 channel_keys 为空 / URL 解析失败被跳过)。页面不渲染
/// createdAt/updatedAt,由 serde 忽略未知字段。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyNodeReportRow {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url_masked: String,
    #[serde(default)]
    pub channel_keys: Vec<String>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub remark: String,
    #[serde(default)]
    pub stats: Option<NodeRuntimeStats>,
}

/// `GET /api/proxy_nodes/report` 响应信封:`{ "items": [...] }`。
///
/// 该端点返回裸 JSON,没有 success/message 信封字段,`ApiClient` 的
/// Envelope 探测必然失败并回落到裸 JSON 解码,因此这里直接映射 items。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct ProxyReportResponse {
    #[serde(default)]
    pub items: Vec<ProxyNodeReportRow>,
}

/// 冷却剩余秒数 → 展示串;0 表示未进入冷却(后端语义),显示占位符不冒充计时。
fn format_cooldown(secs: u64) -> String {
    if secs == 0 {
        "—".to_string()
    } else {
        format!("{secs}秒")
    }
}

/// 最近延迟 → 展示串;None(从未探测/未装配)显示占位符,不拿 0 冒充。
fn format_delay(ms: Option<u16>) -> String {
    ms.map(|v| format!("{v}ms"))
        .unwrap_or_else(|| "—".to_string())
}

/// 代理节点运行态面板:消费 `GET /api/proxy_nodes/report`,把网关数据面
/// (ProxyManager)每节点的 inflight/失败计数/冷却剩余/最近延迟 join DB 行
/// 呈现出来。三态诚实:loading 占位、错误透出后端信息、空态明示无代理节点;
/// stats 缺失的行(停用/未装配)明示"无运行态",不拿全 0 冒充。
#[component]
pub fn ProxyRuntimePanel() -> Element {
    let mut items = use_signal(|| None::<Vec<ProxyNodeReportRow>>);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut reload = use_signal(|| 0u32);

    use_effect(move || {
        // reload 变化(首帧或点击刷新)触发重新拉取
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared();
            match client
                .get::<ProxyReportResponse>("/api/proxy_nodes/report")
                .await
            {
                Ok(v) => {
                    items.set(Some(v.items));
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    let nodes = items().unwrap_or_default();
    let loading = loading();
    let err = err();

    rsx! {
        section {
            role: "region",
            "aria-label": "代理节点运行态",
            id: "proxy-runtime-section",
            "data-testid": "proxy-runtime-panel",
            class: "scroll-mt-8 rounded-2xl border border-zinc-800 bg-zinc-900/60 p-6 space-y-6",

            div { class: "flex items-center justify-between",
                div { class: "flex items-center gap-2",
                    h2 { class: "text-lg font-semibold text-zinc-100", "代理节点运行态" }
                    span { class: "text-xs text-zinc-500", "网关数据面实时采集" }
                }
                button {
                    class: "shrink-0 rounded-xl border border-zinc-700 px-3 py-2 text-xs text-zinc-300 transition-colors hover:bg-zinc-800",
                    "data-testid": "proxy-runtime-refresh",
                    onclick: move |_| reload.set(reload() + 1),
                    "刷新"
                }
            }

            if let Some(e) = err {
                div {
                    role: "alert",
                    "data-testid": "proxy-runtime-error",
                    class: "rounded-xl border border-red-500/30 bg-red-950/30 p-4 text-sm text-red-400",
                    "加载代理节点运行态失败:{e}"
                }
            } else if loading {
                div {
                    "data-testid": "proxy-runtime-loading",
                    class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-10 text-center",
                    p { class: "text-zinc-400", "正在加载代理节点运行态…" }
                }
            } else if nodes.is_empty() {
                div {
                    "data-testid": "proxy-runtime-empty",
                    class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-10 text-center",
                    p { class: "text-zinc-400", "无代理节点" }
                    p { class: "mt-1 text-xs text-zinc-600", "先在上方导入出口代理节点,导入成功的节点运行态会在这里展示" }
                }
            } else {
                div {
                    role: "list",
                    "aria-label": "代理节点运行态列表",
                    class: "divide-y divide-zinc-800/80",
                    for node in &nodes {
                        div {
                            role: "listitem",
                            "aria-label": "{node.name}",
                            class: "flex flex-wrap items-center justify-between gap-x-4 gap-y-1 py-3",
                            div { class: "min-w-0",
                                div { class: "flex items-center gap-2",
                                    span { class: "text-sm text-zinc-200", "{node.name}" }
                                    span {
                                        class: "rounded-full bg-zinc-800 px-2 py-0.5 text-xs text-zinc-400",
                                        if node.enabled { "已启用" } else { "已停用" }
                                    }
                                }
                                p { class: "mt-0.5 truncate text-xs font-mono text-zinc-500", "{node.url_masked}" }
                            }
                            if let Some(s) = &node.stats {
                                div { class: "flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-zinc-400",
                                    span { "在途 {s.inflight}" }
                                    span { "失败 {s.failure_count}" }
                                    span { "冷却 {format_cooldown(s.cooldown_remaining_secs)}" }
                                    span { "延迟 {format_delay(s.last_delay_ms)}" }
                                }
                            } else {
                                div { class: "text-xs text-zinc-500", "无运行态(未启用或未装配)" }
                            }
                        }
                    }
                }
            }
        }
    }
}
