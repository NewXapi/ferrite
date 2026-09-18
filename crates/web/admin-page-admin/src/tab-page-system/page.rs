//! 系统页:组合系统诊断区与代理节点区。
//! - 拉取 `/api/system-info`(admin-ops system_info),派生三组渲染数据:
//!   概览统计卡、实体统计卡、运行环境明细行
//! - 挂载三个面板:`SystemOptionsPanel`(站点选项)、`ProxyNodesPanel`
//!   (出口代理节点导入)、`ProxyRuntimePanel`(代理节点运行态)
//! - 所有数据来自后端实时采集;后端不提供的站点配置/功能开关字段已移除,
//!   不再使用 EntityStore mock 假数据。
//!
//! DTO 与格式化辅助见 `shared`,面板实现见各自文件。

use dioxus::prelude::*;

use super::options::SystemOptionsPanel;
use super::proxy_nodes::ProxyNodesPanel;
use super::proxy_runtime::ProxyRuntimePanel;
use super::shared::{
    SEC_COUNTS, SEC_ENV, SEC_STATS, SystemInfoView, format_bytes, format_db_status, format_load,
    format_started_at, format_uptime,
};
use crate::tab_page_groups::StatCard;
use client::ApiClient;

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

            // 4. 站点选项(key/value 平表,真实 /api/option 读写)
            SystemOptionsPanel {}

            // 5. 出口代理节点导入面板 (M2-C)
            ProxyNodesPanel {}

            // 5. 代理节点运行态面板 (M3,消费 GET /api/proxy_nodes/report)
            ProxyRuntimePanel {}
        }
    }
}
