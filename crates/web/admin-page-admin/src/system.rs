//! 系统页:接入真实 `/api/system-info`(admin-ops system_info)的系统诊断面板。
//! 包含:顶部运行指标概览(运行时长/内存/CPU/数据库/进程内存)、核心实体统计、
//! 运行环境明细行。所有数据来自后端实时采集;后端不提供的站点配置/功能开关
//! 字段已移除,不再使用 EntityStore mock 假数据。

use dioxus::prelude::*;

use crate::groups::StatCard;
use client::ApiClient;

const SEC_STATS: &str = "系统概览";
const SEC_COUNTS: &str = "实体统计";
const SEC_ENV: &str = "运行环境";

// ---------- 本地 DTO ----------
// 只声明页面渲染所需的字段;serde 默认忽略未知字段,数值字段带 default
// 防御个别字段缺失导致整个面板解码失败。

/// 系统综合信息视图,对应后端 `admin_ops::system_info::SystemInfoView`。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SystemInfoView {
    #[serde(default)]
    runtime: RuntimeInfo,
    #[serde(default)]
    uptime: UptimeInfo,
    #[serde(default)]
    memory: MemoryInfo,
    #[serde(default)]
    cpu: CpuInfo,
    #[serde(default)]
    database: DatabaseInfo,
    #[serde(default)]
    counts: EntityCounts,
}

/// 运行时基础环境。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeInfo {
    #[serde(default)]
    version: String,
    #[serde(default)]
    os: String,
    #[serde(default)]
    arch: String,
    #[serde(default)]
    hostname: String,
}

/// 进程启动与运行时间。`started_at` 是后端 DateTime<Utc> 序列化的 RFC3339 串。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UptimeInfo {
    #[serde(default)]
    started_at: String,
    #[serde(default)]
    uptime_seconds: u64,
}

/// 内存监控数据(字节)。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoryInfo {
    #[serde(default)]
    process_rss_bytes: u64,
    #[serde(default)]
    system_total_bytes: u64,
    #[serde(default)]
    system_used_bytes: u64,
    #[serde(default)]
    system_available_bytes: u64,
}

/// CPU 核心数与系统负载。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CpuInfo {
    #[serde(default)]
    num_cpus: usize,
    #[serde(default)]
    load_avg_1m: Option<f64>,
    #[serde(default)]
    load_avg_5m: Option<f64>,
    #[serde(default)]
    load_avg_15m: Option<f64>,
}

/// 数据库连接池诊断。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DatabaseInfo {
    #[serde(default)]
    status: String,
    #[serde(default)]
    pool_size: u32,
    #[serde(default)]
    idle_connections: u32,
}

/// 核心业务实体数量统计。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct EntityCounts {
    #[serde(default)]
    users: i64,
    #[serde(default)]
    channels: i64,
    #[serde(default)]
    active_channels: i64,
    #[serde(default)]
    models: i64,
    #[serde(default)]
    tokens: i64,
}

/// 系统页:顶部运行指标 + 实体统计 + 运行环境明细,数据来自 `/api/system-info`。
#[component]
pub fn SystemPage() -> Element {
    let mut info = use_signal(|| None::<SystemInfoView>);
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
            match client.get::<SystemInfoView>("/api/system-info").await {
                Ok(v) => {
                    info.set(Some(v));
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    let data = info();
    let loading = loading();
    let err = err();

    // 概览统计卡:运行时长 / 内存占用 / CPU 负载 / 数据库 / 进程内存
    let stats: Vec<(String, &'static str)> = match &data {
        Some(v) => vec![
            (format_uptime(v.uptime.uptime_seconds), "运行时长"),
            (
                format!(
                    "{}/{}",
                    format_bytes(v.memory.system_used_bytes),
                    format_bytes(v.memory.system_total_bytes)
                ),
                "系统内存 (已用/总量)",
            ),
            (
                format!("{} · {}核", format_load(v.cpu.load_avg_1m), v.cpu.num_cpus),
                "CPU 负载 (1m)",
            ),
            (format_db_status(&v.database.status), "数据库状态"),
            (format_bytes(v.memory.process_rss_bytes), "进程常驻内存"),
        ],
        None => Vec::new(),
    };

    // 实体统计卡:核心业务实体行数
    let count_cards: Vec<(String, &'static str)> = match &data {
        Some(v) => vec![
            (v.counts.users.to_string(), "注册用户"),
            (v.counts.channels.to_string(), "渠道总数"),
            (v.counts.active_channels.to_string(), "启用渠道"),
            (v.counts.models.to_string(), "模型数量"),
            (v.counts.tokens.to_string(), "Token 总数"),
        ],
        None => Vec::new(),
    };

    // 运行环境明细行
    let env_rows: Vec<(String, String)> = match &data {
        Some(v) => vec![
            ("服务版本".to_string(), v.runtime.version.clone()),
            (
                "操作系统".to_string(),
                format!("{} / {}", v.runtime.os, v.runtime.arch),
            ),
            ("主机名".to_string(), v.runtime.hostname.clone()),
            (
                "启动时间".to_string(),
                format_started_at(&v.uptime.started_at),
            ),
            (
                "负载均值".to_string(),
                format!(
                    "1m {} · 5m {} · 15m {}",
                    format_load(v.cpu.load_avg_1m),
                    format_load(v.cpu.load_avg_5m),
                    format_load(v.cpu.load_avg_15m)
                ),
            ),
            (
                "连接池".to_string(),
                format!(
                    "大小 {} · 空闲 {}",
                    v.database.pool_size, v.database.idle_connections
                ),
            ),
            (
                "内存明细".to_string(),
                format!(
                    "已用 {} · 可用 {} · 进程 {}",
                    format_bytes(v.memory.system_used_bytes),
                    format_bytes(v.memory.system_available_bytes),
                    format_bytes(v.memory.process_rss_bytes)
                ),
            ),
        ],
        None => Vec::new(),
    };

    rsx! {
        div { class: "flex flex-col gap-6",

            // 1. 系统概览统计区(真实 /api/system-info 采集)
            section {
                id: "system-sec-stats",
                "data-testid": "system-panel",
                class: "scroll-mt-8 space-y-3",
                div { class: "flex items-center justify-between",
                    h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
                    button {
                        class: "shrink-0 rounded-xl border border-zinc-700 px-3 py-2 text-xs text-zinc-300 transition-colors hover:bg-zinc-800",
                        "data-testid": "refresh-system",
                        onclick: move |_| reload.set(reload() + 1),
                        "刷新"
                    }
                }
                if let Some(e) = err {
                    div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 px-4 py-6 text-center",
                        p { class: "text-sm text-red-300", "加载系统信息失败" }
                        p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                        button {
                            class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                            onclick: move |_| reload.set(reload() + 1),
                            "重试"
                        }
                    }
                } else if loading {
                    div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-10 text-center",
                        p { class: "text-zinc-400", "正在加载系统信息…" }
                    }
                } else if stats.is_empty() {
                    div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-10 text-center",
                        p { class: "text-zinc-400", "暂无系统信息" }
                        p { class: "mt-1 text-xs text-zinc-600", "后端未返回采集数据 —— 服务重启产生指标后这里会展示真实系统状态" }
                    }
                } else {
                    div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                        for (value, label) in stats {
                            StatCard { value, label }
                        }
                    }
                }
            }

            // 2. 实体统计(核心业务实体行数)
            if !count_cards.is_empty() {
                section { id: "system-sec-counts", class: "scroll-mt-8 space-y-3",
                    h2 { class: "text-lg font-medium text-zinc-100", "{SEC_COUNTS}" }
                    div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                        for (value, label) in count_cards {
                            StatCard { value, label }
                        }
                    }
                }
            }

            // 3. 运行环境明细行
            if !env_rows.is_empty() {
                section {
                    id: "system-sec-env",
                    class: "scroll-mt-8 rounded-xl border border-zinc-800 bg-zinc-900/60 p-5 space-y-4",
                    div {
                        h2 { class: "text-sm font-medium text-zinc-200", "{SEC_ENV}" }
                        p { class: "text-xs text-zinc-500", "采集自服务端进程与数据库连接池的实时诊断数据" }
                    }
                    div { class: "divide-y divide-zinc-800/80",
                        for (label, value) in env_rows {
                            div { class: "flex items-center justify-between gap-4 py-2.5",
                                span { class: "shrink-0 text-xs text-zinc-500", "{label}" }
                                span { class: "break-all text-right text-xs font-mono text-zinc-300", "{value}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 秒数 → 人类可读运行时长(天/小时/分,不足一分钟时显示秒)。
fn format_uptime(total_secs: u64) -> String {
    let days = total_secs / 86_400;
    let hours = (total_secs % 86_400) / 3_600;
    let mins = (total_secs % 3_600) / 60;
    let secs = total_secs % 60;
    if days > 0 {
        format!("{days}天 {hours}小时")
    } else if hours > 0 {
        format!("{hours}小时 {mins}分")
    } else if mins > 0 {
        format!("{mins}分 {secs}秒")
    } else {
        format!("{secs}秒")
    }
}

/// 字节数 → 人类可读容量(1 位小数,自动选 GB/MB/KB/B)。
fn format_bytes(bytes: u64) -> String {
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.1}GB", b / GB)
    } else if b >= MB {
        format!("{:.1}MB", b / MB)
    } else if b >= KB {
        format!("{:.1}KB", b / KB)
    } else {
        format!("{bytes}B")
    }
}

/// 负载均值 → 两位小数;None(平台不提供)显示占位符,不造数据。
fn format_load(v: Option<f64>) -> String {
    v.map(|x| format!("{x:.2}")).unwrap_or_else(|| "—".into())
}

/// 数据库连通状态码 → 中文标签(未知值原样展示)。
fn format_db_status(status: &str) -> String {
    match status {
        "connected" => "已连接".to_string(),
        "degraded" => "已降级".to_string(),
        other => other.to_string(),
    }
}

/// RFC3339 启动时间 → 去掉小数秒的可读时间串。
fn format_started_at(rfc3339: &str) -> String {
    rfc3339.split('.').next().unwrap_or(rfc3339).to_string()
}
