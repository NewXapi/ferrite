//! 系统页共享类型与辅助:
//! - `/api/system-info` 的本地 DTO(只声明页面渲染所需的字段;serde 默认忽略
//!   未知字段,数值字段带 default,防御个别字段缺失导致整个面板解码失败)
//! - 概览/实体统计/运行环境三个区段的文案常量
//! - 运行时长/字节/负载/数据库状态/启动时间的纯函数格式化(不造数据,
//!   缺失值显示占位符)
//!
//! 这些是页面与面板之间的共享层,不跨 crate 暴露(mod.rs 不再导出内部 DTO)。

/// 系统综合信息视图,对应后端 `admin_ops::system_info::SystemInfoView`。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfoView {
    #[serde(default)]
    pub runtime: RuntimeInfo,
    #[serde(default)]
    pub uptime: UptimeInfo,
    #[serde(default)]
    pub memory: MemoryInfo,
    #[serde(default)]
    pub cpu: CpuInfo,
    #[serde(default)]
    pub database: DatabaseInfo,
    #[serde(default)]
    pub counts: EntityCounts,
}

/// 运行时基础环境。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub os: String,
    #[serde(default)]
    pub arch: String,
    #[serde(default)]
    pub hostname: String,
}

/// 进程启动与运行时间。`started_at` 是后端 DateTime<Utc> 序列化的 RFC3339 串。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UptimeInfo {
    #[serde(default)]
    pub started_at: String,
    #[serde(default)]
    pub uptime_seconds: u64,
}

/// 内存监控数据(字节)。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
    #[serde(default)]
    pub process_rss_bytes: u64,
    #[serde(default)]
    pub system_total_bytes: u64,
    #[serde(default)]
    pub system_used_bytes: u64,
    #[serde(default)]
    pub system_available_bytes: u64,
}

/// CPU 核心数与系统负载。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    #[serde(default)]
    pub num_cpus: usize,
    #[serde(default)]
    pub load_avg_1m: Option<f64>,
    #[serde(default)]
    pub load_avg_5m: Option<f64>,
    #[serde(default)]
    pub load_avg_15m: Option<f64>,
}

/// 数据库连接池诊断。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseInfo {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub pool_size: u32,
    #[serde(default)]
    pub idle_connections: u32,
}

/// 核心业务实体数量统计。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityCounts {
    #[serde(default)]
    pub users: i64,
    #[serde(default)]
    pub channels: i64,
    #[serde(default)]
    pub active_channels: i64,
    #[serde(default)]
    pub models: i64,
    #[serde(default)]
    pub tokens: i64,
}

/// 系统概览区段标题。
pub const SEC_STATS: &str = "系统概览";
/// 实体统计区段标题。
pub const SEC_COUNTS: &str = "实体统计";
/// 运行环境区段标题。
pub const SEC_ENV: &str = "运行环境";

/// 秒数 → 人类可读运行时长(天/小时/分,不足一分钟时显示秒)。
pub fn format_uptime(total_secs: u64) -> String {
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
pub fn format_bytes(bytes: u64) -> String {
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
pub fn format_load(v: Option<f64>) -> String {
    v.map(|x| format!("{x:.2}")).unwrap_or_else(|| "—".into())
}

/// 数据库连通状态码 → 中文标签(未知值原样展示)。
pub fn format_db_status(status: &str) -> String {
    match status {
        "connected" => "已连接".to_string(),
        "degraded" => "已降级".to_string(),
        other => other.to_string(),
    }
}

/// RFC3339 启动时间 → 去掉小数秒的可读时间串。
pub fn format_started_at(rfc3339: &str) -> String {
    rfc3339.split('.').next().unwrap_or(rfc3339).to_string()
}
