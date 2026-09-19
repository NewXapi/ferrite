//! 系统页共享类型与辅助:
//! - `/api/system-info` 的本地 DTO(只声明页面渲染所需的字段;serde 默认忽略
//!   未知字段,数值字段带 default,防御个别字段缺失导致整个面板解码失败)
//! - 概览/实体统计/运行环境三个区段的文案常量
//! - 运行时长/字节/负载/数据库状态/启动时间的纯函数格式化(不造数据,
//!   缺失值显示占位符)
//!
//! 本文件同时承载该 tab 的全部用户可见文案常量（i18n 第 1 层）：
//! 常量值即原字面量，逐字符保持一致以保证零渲染变化。
//!
//! 边界:不放组件(`#[component]` 在 `page` / `overview` / `options` /
//! `proxy_nodes` / `proxy_runtime` 里),不放网络调用(在 `page` 与各面板的
//! effect 里);这里只有 DTO 定义、纯格式化函数与文案常量。
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

// ============ 用户可见文案常量（i18n 第 1 层） ============

// ---- SEC_* : 区段标题 / 说明条 ----

/// 系统概览区段标题。
pub const SEC_STATS: &str = "系统概览";
/// 实体统计区段标题。
pub const SEC_COUNTS: &str = "实体统计";
/// 运行环境区段标题。
pub const SEC_ENV: &str = "运行环境";
/// 运行环境区段说明:数据来源。
pub const SEC_ENV_NOTE: &str = "采集自服务端进程与数据库连接池的实时诊断数据";
/// 站点选项区段说明。
pub const SEC_OPTIONS_NOTE: &str = "运行时选项 (key/value 平表),来自 /api/option 注册表与数据库值";
/// 出口代理节点导入面板的 aria 区域名。
pub const SEC_PROXY_IMPORT: &str = "出口代理节点导入";
/// 代理节点运行态面板的 aria 区域名。
pub const SEC_PROXY_RUNTIME: &str = "代理节点运行态";
/// 代理节点运行态列表的 aria 区域名。
pub const SEC_PROXY_RUNTIME_LIST: &str = "代理节点运行态列表";
/// 代理节点运行态区段说明:数据来源。
pub const SEC_PROXY_RUNTIME_NOTE: &str = "网关数据面实时采集";

// ---- LBL_* : 标签 / 表头 / 字段旁的说明 ----

/// 概览卡:运行时长。
pub const LBL_UPTIME: &str = "运行时长";
/// 概览卡:系统内存。
pub const LBL_SYS_MEMORY: &str = "系统内存 (已用/总量)";
/// 概览卡:CPU 负载。
pub const LBL_CPU_LOAD: &str = "CPU 负载 (1m)";
/// 概览卡:数据库状态。
pub const LBL_DB_STATUS: &str = "数据库状态";
/// 概览卡:进程常驻内存。
pub const LBL_PROCESS_RSS: &str = "进程常驻内存";
/// 实体统计卡:注册用户。
pub const LBL_COUNT_USERS: &str = "注册用户";
/// 实体统计卡:渠道总数。
pub const LBL_COUNT_CHANNELS: &str = "渠道总数";
/// 实体统计卡:启用渠道。
pub const LBL_COUNT_ACTIVE_CHANNELS: &str = "启用渠道";
/// 实体统计卡:模型数量。
pub const LBL_COUNT_MODELS: &str = "模型数量";
/// 实体统计卡:Token 总数。
pub const LBL_COUNT_TOKENS: &str = "Token 总数";
/// 环境明细行:服务版本。
pub const LBL_SERVICE_VERSION: &str = "服务版本";
/// 环境明细行:操作系统。
pub const LBL_OS: &str = "操作系统";
/// 环境明细行:主机名。
pub const LBL_HOSTNAME: &str = "主机名";
/// 环境明细行:启动时间。
pub const LBL_STARTED_AT: &str = "启动时间";
/// 环境明细行:负载均值。
pub const LBL_LOAD_AVG: &str = "负载均值";
/// 环境明细行:连接池。
pub const LBL_CONN_POOL: &str = "连接池";
/// 环境明细行:内存明细。
pub const LBL_MEMORY_DETAIL: &str = "内存明细";
/// 站点选项面板标题(同时用作局部变量名源)。
pub const LBL_SITE_OPTIONS: &str = "站点选项";
/// 出口代理节点面板标题。
pub const LBL_PROXY_NODES: &str = "出口代理节点";
/// 代理节点运行态面板标题。
pub const LBL_PROXY_RUNTIME: &str = "代理节点运行态";
/// 代理节点状态徽标:已启用。
pub const LBL_NODE_ENABLED: &str = "已启用";
/// 代理节点状态徽标:已停用。
pub const LBL_NODE_DISABLED: &str = "已停用";

// ---- BTN_* : 按钮文案 ----

/// 系统概览区刷新按钮。
pub const BTN_REFRESH: &str = "刷新";
/// 系统概览区错误态重试按钮。
pub const BTN_RETRY: &str = "重试";
/// 出口代理节点导入按钮。
pub const BTN_IMPORT: &str = "导入";
/// 出口代理节点导入按钮的加载态文案。
pub const BTN_IMPORTING: &str = "导入中...";

// ---- FIELD_* : 表单字段标签 ----

/// 出口代理节点导入:订阅链接输入框占位。
pub const FIELD_SUB_URL_PLACEHOLDER: &str = "https://example.com/clash.yaml";
/// 出口代理节点导入:渠道列表输入框占位。
pub const FIELD_CHANNELS_PLACEHOLDER: &str = "openai,claude（逗号分隔）";
/// 出口代理节点导入:优先级输入框占位。
pub const FIELD_PRIORITY_PLACEHOLDER: &str = "优先级 (默认 10)";
/// 出口代理节点导入:订阅导入区标题。
pub const FIELD_SUB_IMPORT: &str = "订阅导入";
/// 出口代理节点导入:粘贴分享链接区标题。
pub const FIELD_SHARE_IMPORT: &str = "粘贴分享链接";

// ---- MSG_* : 提示 / 错误 / 空态 / 占位 ----

/// 出口代理节点导入:URL 与渠道均必填的校验提示。
pub const MSG_IMPORT_URL_REQUIRED: &str = "URL 和渠道不能为空";
/// 出口代理节点导入:分享链接与渠道均必填的校验提示。
pub const MSG_IMPORT_SHARE_REQUIRED: &str = "分享链接和渠道不能为空";
/// 出口代理节点导入:已创建计数前缀。
pub const MSG_CREATED_PREFIX: &str = "已创建: ";
/// 出口代理节点导入:已跳过计数前缀。
pub const MSG_SKIPPED_PREFIX: &str = "已跳过: ";
/// 出口代理节点导入:失败明细标题。
pub const MSG_FAILURES_TITLE: &str = "失败明细（本功能核心卖点）：";
/// 出口代理节点导入:全部成功提示。
pub const MSG_ALL_IMPORTED: &str = "所有节点导入成功";
/// 代理节点运行态:加载失败前缀(后接错误详情)。
pub const MSG_RUNTIME_LOAD_FAILED: &str = "加载代理节点运行态失败:";
/// 代理节点运行态:加载态占位。
pub const MSG_RUNTIME_LOADING: &str = "正在加载代理节点运行态…";
/// 代理节点运行态:空态标题。
pub const MSG_RUNTIME_EMPTY: &str = "无代理节点";
/// 代理节点运行态:空态说明。
pub const MSG_RUNTIME_EMPTY_HINT: &str =
    "先在上方导入出口代理节点,导入成功的节点运行态会在这里展示";
/// 代理节点运行态:该行无运行态时的说明。
pub const MSG_NO_RUNTIME: &str = "无运行态(未启用或未装配)";
/// 代理节点运行态:在途数标签前缀。
pub const MSG_INFLIGHT_PREFIX: &str = "在途 ";
/// 代理节点运行态:失败数标签前缀。
pub const MSG_FAILURE_PREFIX: &str = "失败 ";
/// 代理节点运行态:冷却剩余标签前缀。
pub const MSG_COOLDOWN_PREFIX: &str = "冷却 ";
/// 代理节点运行态:最近延迟标签前缀。
pub const MSG_DELAY_PREFIX: &str = "延迟 ";
/// 系统概览:加载失败标题。
pub const MSG_LOAD_FAILED: &str = "加载系统信息失败";
/// 系统概览:加载态占位。
pub const MSG_LOADING: &str = "正在加载系统信息…";
/// 系统概览:空态标题。
pub const MSG_EMPTY: &str = "暂无系统信息";
/// 系统概览:空态说明。
pub const MSG_EMPTY_HINT: &str = "后端未返回采集数据 —— 服务重启产生指标后这里会展示真实系统状态";
/// 站点选项:加载态占位。
pub const MSG_OPTIONS_LOADING: &str = "正在加载站点选项…";
/// 站点选项:空态占位。
pub const MSG_OPTIONS_EMPTY: &str = "暂无站点选项";

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
