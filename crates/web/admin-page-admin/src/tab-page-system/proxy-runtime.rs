//! 代理节点运行态面板 (M3):消费 `GET /api/proxy_nodes/report`,把网关数据面
//! (ProxyManager)每节点的 inflight/失败计数/冷却剩余/最近延迟 join DB 行
//! 呈现出来。三态诚实:loading 占位、错误透出后端信息、空态明示无代理节点;
//! stats 缺失的行(停用/未装配)明示"无运行态",不拿全 0 冒充。

use dioxus::prelude::*;

use super::shared::{
    BTN_REFRESH, LBL_NODE_DISABLED, LBL_NODE_ENABLED, LBL_PROXY_RUNTIME, MSG_COOLDOWN_PREFIX,
    MSG_DELAY_PREFIX, MSG_FAILURE_PREFIX, MSG_INFLIGHT_PREFIX, MSG_NO_RUNTIME, MSG_RUNTIME_EMPTY,
    MSG_RUNTIME_EMPTY_HINT, MSG_RUNTIME_LOAD_FAILED, MSG_RUNTIME_LOADING, SEC_PROXY_RUNTIME,
    SEC_PROXY_RUNTIME_LIST, SEC_PROXY_RUNTIME_NOTE,
};
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
///
/// 【是什么】代理节点运行态列表面板:区段头(标题 + 数据来源说明 + 刷新)+
/// 每节点一行的运行指标。
///
/// 【做什么】挂载时拉 `/api/proxy_nodes/report`,按 `err` / `loading` /
/// `nodes.is_empty()` 渲染错误 / 加载 / 空 / 列表四态;有数据的行 show
/// name + 启停徽标 + 掩码 URL + 在途/失败/冷却/延迟四项指标(`stats` 为
/// `None` 时改显「无运行态」)。不负责导入节点(在 `proxy_nodes`)、
/// 不负责节点的编辑/删除(写路径在拓扑抽屉)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点「刷新」→ `reload` signal 递增,页面 effect 重拉 report(一次 GET)。
/// 数据交互:仅挂载与点刷新时各发一次 GET;无写请求。
///
/// 【样式】外壳 `section#proxy-runtime-section` 为 `scroll-mt-8 rounded-2xl
/// border border-border bg-card/60 p-6 space-y-6`;区段头 h2
/// `text-lg font-semibold` + 副说明 `text-xs text-muted-foreground`,右侧描边刷新按钮;
/// 错误态红底圆角卡;加载/空态虚线描边;列表 `divide-y divide-zinc-800/80`,
/// 每行 `flex flex-wrap items-center justify-between gap-x-4`,行内指标
/// `text-xs text-muted-foreground`,状态徽标 `rounded-full bg-secondary`。
///
/// 【子组件组成】无子组件:全部为原生 dioxus 元素(`section` / `div` / `h2`
/// / `span` / `p` / `button`)。
///
/// 【数据流】
/// - 对内(入):无 props;`items` / `loading` / `err` / `reload` 四个 signal
///   均为组件内 `use_signal`,由挂载 effect 填充。
/// - 对外(出):无 EventHandler / 无 signal 写回;刷新只改本组件 `reload`。
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
            "aria-label": SEC_PROXY_RUNTIME,
            id: "proxy-runtime-section",
            "data-testid": "proxy-runtime-panel",
            class: "scroll-mt-8 rounded-2xl border border-border bg-card/60 p-6 space-y-6",

            div { class: "flex items-center justify-between",
                div { class: "flex items-center gap-2",
                    h2 { class: "{ui::TYPE_TITLE}", {LBL_PROXY_RUNTIME} }
                    span { class: "{ui::TYPE_DESC}", {SEC_PROXY_RUNTIME_NOTE} }
                }
                button {
                    class: "shrink-0 rounded-xl border border-border px-3 py-2 {ui::TYPE_DESC} transition-colors hover:bg-secondary",
                    "data-testid": "proxy-runtime-refresh",
                    onclick: move |_| reload.set(reload() + 1),
                    {BTN_REFRESH}
                }
            }

            if let Some(e) = err {
                div {
                    role: "alert",
                    "data-testid": "proxy-runtime-error",
                    class: "rounded-xl border border-destructive bg-destructive p-4 {ui::TYPE_BODY} {ui::C_DANGER}",
                    {MSG_RUNTIME_LOAD_FAILED} "{e}"
                }
            } else if loading {
                div {
                    "data-testid": "proxy-runtime-loading",
                    class: "rounded-2xl border border-dashed border-border bg-card/50 py-10 text-center",
                    p { class: "{ui::C_MUTED}", {MSG_RUNTIME_LOADING} }
                }
            } else if nodes.is_empty() {
                div {
                    "data-testid": "proxy-runtime-empty",
                    class: "rounded-2xl border border-dashed border-border bg-card/50 py-10 text-center",
                    p { class: "{ui::C_MUTED}", {MSG_RUNTIME_EMPTY} }
                    p { class: "mt-1 {ui::TYPE_DESC}", {MSG_RUNTIME_EMPTY_HINT} }
                }
            } else {
                div {
                    role: "list",
                    "aria-label": SEC_PROXY_RUNTIME_LIST,
                    class: "divide-y divide-zinc-800/80",
                    for node in &nodes {
                        div {
                            role: "listitem",
                            "aria-label": "{node.name}",
                            class: "flex flex-wrap items-center justify-between gap-x-4 gap-y-1 py-3",
                            div { class: "min-w-0",
                                div { class: "flex items-center gap-2",
                                    span { class: "{ui::TYPE_BODY}", "{node.name}" }
                                    span {
                                        class: "rounded-full bg-secondary px-2 py-0.5 {ui::TYPE_DESC}",
                                        if node.enabled { {LBL_NODE_ENABLED} } else { {LBL_NODE_DISABLED} }
                                    }
                                }
                                p { class: "mt-0.5 truncate text-xs font-mono text-muted-foreground", "{node.url_masked}" }
                            }
                            if let Some(s) = &node.stats {
                                div { class: "flex flex-wrap items-center gap-x-4 gap-y-1 {ui::TYPE_DESC}",
                                    span { {MSG_INFLIGHT_PREFIX} "{s.inflight}" }
                                    span { {MSG_FAILURE_PREFIX} "{s.failure_count}" }
                                    span { {MSG_COOLDOWN_PREFIX} "{format_cooldown(s.cooldown_remaining_secs)}" }
                                    span { {MSG_DELAY_PREFIX} "{format_delay(s.last_delay_ms)}" }
                                }
                            } else {
                                div { class: "{ui::TYPE_DESC}", {MSG_NO_RUNTIME} }
                            }
                        }
                    }
                }
            }
        }
    }
}
