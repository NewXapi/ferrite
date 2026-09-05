//! 系统诊断与指标采集服务。
//!
//! 提供系统运行环境、进程 Uptime、内存/CPU 资源占用以及数据库连通与实体统计。
//! 参考: new-api (`/system-info`) + sub2api (`/api/v1/admin/dashboard/stats`)。

use std::sync::Arc;
use std::time::Instant;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::get;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use auth::error::AuthError;
use auth::routes::bearer_user;
use auth::service::AuthService;

/// 进程启动时间记录器。
#[derive(Debug, Clone)]
pub struct ProcessTimeTracker {
    pub started_at: DateTime<Utc>,
    pub start_instant: Instant,
}

impl Default for ProcessTimeTracker {
    fn default() -> Self {
        Self {
            started_at: Utc::now(),
            start_instant: Instant::now(),
        }
    }
}

/// 系统综合信息视图。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfoView {
    pub version: String,
    pub os: String,
    pub arch: String,
    pub hostname: String,
    pub started_at: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub memory: MemoryInfo,
    pub cpu: CpuInfo,
    pub database: DatabaseInfo,
    pub counts: EntityCounts,
}

/// 内存监控数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
    pub process_rss_bytes: u64,
    pub process_virt_bytes: u64,
    pub system_total_bytes: u64,
    pub system_used_bytes: u64,
    pub system_available_bytes: u64,
}

/// CPU 与系统负载数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    pub num_cpus: usize,
    pub load_avg_1m: Option<f64>,
    pub load_avg_5m: Option<f64>,
    pub load_avg_15m: Option<f64>,
}

/// 数据库连接池诊断数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseInfo {
    pub status: String,
    pub pool_size: u32,
    pub idle_connections: u32,
}

/// 核心业务实体数量统计。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityCounts {
    pub users: i64,
    pub channels: i64,
    pub active_channels: i64,
    pub models: i64,
    pub tokens: i64,
}

/// 系统信息采集服务。
pub struct SystemInfoService {
    pool: PgPool,
    tracker: ProcessTimeTracker,
}

impl SystemInfoService {
    pub fn new(pool: PgPool, tracker: ProcessTimeTracker) -> Self {
        Self { pool, tracker }
    }

    /// 采集完整系统指标视图。
    pub async fn get_system_info(&self) -> Result<SystemInfoView, AuthError> {
        let uptime_seconds = self.tracker.start_instant.elapsed().as_secs();
        let memory = collect_memory_info();
        let cpu = collect_cpu_info();
        let (database, counts) = self.collect_database_and_counts().await?;

        Ok(SystemInfoView {
            version: env!("CARGO_PKG_VERSION").to_string(),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            hostname: collect_hostname(),
            started_at: self.tracker.started_at,
            uptime_seconds,
            memory,
            cpu,
            database,
            counts,
        })
    }

    async fn collect_database_and_counts(&self) -> Result<(DatabaseInfo, EntityCounts), AuthError> {
        let row: (i64, i64, i64, i64, i64) = sqlx::query_as(
            r#"SELECT
                (SELECT count(*) FROM auth_users),
                (SELECT count(*) FROM api_channels),
                (SELECT count(*) FROM api_channels WHERE status = 1),
                (SELECT count(*) FROM api_models),
                (SELECT count(*) FROM api_tokens)"#,
        )
        .fetch_one(&self.pool)
        .await?;

        let database = DatabaseInfo {
            status: "connected".to_string(),
            pool_size: self.pool.size(),
            idle_connections: self.pool.num_idle() as u32,
        };

        let counts = EntityCounts {
            users: row.0,
            channels: row.1,
            active_channels: row.2,
            models: row.3,
            tokens: row.4,
        };

        Ok((database, counts))
    }
}

/// 采集主机名。
fn collect_hostname() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .or_else(|_| std::fs::read_to_string("/etc/hostname"))
        .unwrap_or_else(|_| std::env::var("HOSTNAME").unwrap_or_default())
        .trim()
        .to_string()
}

/// 采集内存信息（支持 Linux /proc 解析，非 Linux 环境降级）。
fn collect_memory_info() -> MemoryInfo {
    let (process_virt_bytes, process_rss_bytes) = parse_process_statm();
    let (system_total_bytes, system_available_bytes) = parse_proc_meminfo();
    let system_used_bytes = system_total_bytes.saturating_sub(system_available_bytes);

    MemoryInfo {
        process_rss_bytes,
        process_virt_bytes,
        system_total_bytes,
        system_used_bytes,
        system_available_bytes,
    }
}

/// 从 `/proc/self/statm` 获取进程虚拟内存与常驻内存。
fn parse_process_statm() -> (u64, u64) {
    if let Ok(content) = std::fs::read_to_string("/proc/self/statm") {
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 2 {
            let page_size = 4096u64;
            let virt = parts[0].parse::<u64>().unwrap_or(0) * page_size;
            let rss = parts[1].parse::<u64>().unwrap_or(0) * page_size;
            return (virt, rss);
        }
    }
    (0, 0)
}

/// 从 `/proc/meminfo` 获取系统内存总数与可用数。
fn parse_proc_meminfo() -> (u64, u64) {
    let mut total = 0u64;
    let mut available = 0u64;

    if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                total = parse_kb_val(rest);
            } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
                available = parse_kb_val(rest);
            }
        }
    }
    (total, available)
}

fn parse_kb_val(s: &str) -> u64 {
    s.trim()
        .trim_end_matches("kB")
        .trim()
        .parse::<u64>()
        .unwrap_or(0)
        * 1024
}

/// 采集 CPU 核心数与负载信息。
fn collect_cpu_info() -> CpuInfo {
    let num_cpus = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1);

    let (load_avg_1m, load_avg_5m, load_avg_15m) = parse_loadavg();

    CpuInfo {
        num_cpus,
        load_avg_1m,
        load_avg_5m,
        load_avg_15m,
    }
}

/// 从 `/proc/loadavg` 读取系统平均负载。
fn parse_loadavg() -> (Option<f64>, Option<f64>, Option<f64>) {
    if let Ok(content) = std::fs::read_to_string("/proc/loadavg") {
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 3 {
            let l1 = parts[0].parse::<f64>().ok();
            let l5 = parts[1].parse::<f64>().ok();
            let l15 = parts[2].parse::<f64>().ok();
            return (l1, l5, l15);
        }
    }
    (None, None, None)
}

// ---------- axum 路由 ----------

#[derive(Clone)]
pub struct SystemInfoAppState {
    pub svc: Arc<SystemInfoService>,
    pub auth: Arc<AuthService>,
}

pub fn router(state: SystemInfoAppState) -> axum::Router {
    axum::Router::new()
        .route("/api/system-info", get(get_system_info_handler))
        .with_state(state)
}

async fn require_admin(auth: &AuthService, h: &HeaderMap) -> Result<Uuid, AuthError> {
    let u = bearer_user(auth, h).await?;
    if u.role >= auth::routes::ADMIN_ROLE_THRESHOLD {
        Uuid::parse_str(&u.key).map_err(|_| AuthError::InvalidToken)
    } else {
        Err(AuthError::Forbidden)
    }
}

type ErrResp = (StatusCode, Json<Value>);

fn err_json(e: AuthError) -> ErrResp {
    (
        e.status(),
        Json(json!({ "code": e.code(), "message": e.to_string() })),
    )
}

async fn get_system_info_handler(
    State(s): State<SystemInfoAppState>,
    h: HeaderMap,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let info = s.svc.get_system_info().await.map_err(err_json)?;
    Ok(Json(json!(info)))
}
