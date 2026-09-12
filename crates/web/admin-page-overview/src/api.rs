//! Dashboard 各页的数据来源。面板只从这里取数,不认识数据是怎么来的。

use client::{ApiClient, ApiResult};
use contract::api::usage::{DashboardSummaryDto, UsageLogPage, UsageStatDto};

/// `/api/models` 列表项的页面本地视图:只映射模型页实际展示的后端 `ModelView` 字段子集。
///
/// 后端没有的字段(价格、六维实力、趋势、分组报价)不在此列——页面不造数据。
/// 容器级 `#[serde(default)]`:后端响应缺任何字段时落 `Default::default()`
/// (数值 0 / bool false / 空串),整条解析不因缺字段失败,页面诚实降级展示。
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModelCardView {
    /// 模型名(公开别名,消费日志按它聚合)。
    pub name: String,
    /// 归属方(owner)。
    pub owner: String,
    /// 模型类型(如 openai / anthropic)。
    pub model_type: String,
    /// 状态:1 = 启用,其余 = 停用(与后端 `status = 1` 判定同口径)。
    pub status: i16,
    /// 累计调用次数。
    pub usage_count: i64,
    /// 最大上下文 tokens。
    pub max_tokens: i32,
    /// 是否支持视觉输入。
    pub is_vision: bool,
    /// 是否支持工具调用。
    pub is_tool: bool,
}

/// 真实调用: GET /api/models?size=100 — 管理端模型列表。
///
/// 返回 `(items, total)`:后端单页上限 100 条(size 被 clamp 到 1..=100),
/// `total` 是库内总数,供页面诚实标注「显示前 N / 共 M 个」。
/// 错误情况:401/403(未登录或非管理员)、网络失败,均走 [`ApiResult`]。
pub async fn list_models_api(client: &ApiClient) -> ApiResult<(Vec<ModelCardView>, i64)> {
    #[derive(Default, serde::Deserialize)]
    struct ModelPage {
        #[serde(default)]
        items: Vec<ModelCardView>,
        #[serde(default)]
        total: i64,
    }
    let r: ModelPage = client.get("/api/models?size=100").await?;
    Ok((r.items, r.total))
}

/// 真实调用: GET /api/dashboard
pub async fn get_dashboard_summary_api(client: &ApiClient) -> ApiResult<DashboardSummaryDto> {
    client.get("/api/dashboard").await
}

/// 真实调用: GET /api/log/stat
pub async fn get_usage_stat_api(client: &ApiClient) -> ApiResult<UsageStatDto> {
    client.get("/api/log/stat").await
}

/// 真实调用: GET /api/log?page=&size=&start=
/// 后端 LogQuery 参数名是 page/size（此前误用 p=/page_size= 导致分页不生效）。
pub async fn list_all_logs_api(
    client: &ApiClient,
    model: Option<&str>,
    page: Option<u32>,
    page_size: Option<u32>,
    start: Option<&str>,
) -> ApiResult<UsageLogPage> {
    let mut query = Vec::new();
    if let Some(m) = model
        && !m.is_empty()
    {
        query.push(format!("model_name={m}"));
    }
    if let Some(p) = page {
        query.push(format!("page={p}"));
    }
    if let Some(ps) = page_size {
        query.push(format!("size={ps}"));
    }
    if let Some(s) = start
        && !s.is_empty()
    {
        query.push(format!("start={s}"));
    }
    let path = if query.is_empty() {
        "/api/log".to_string()
    } else {
        format!("/api/log?{}", query.join("&"))
    };
    client.get(&path).await
}

/// /api/log/top 行：按用户或模型的消耗聚合。
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageTopRow {
    pub name: String,
    pub tokens: i64,
    pub quota: i64,
    pub calls: i64,
}

/// /api/log/trend 行：时间桶 × 模型的用量聚合。
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageTrendRow {
    /// RFC3339 桶起始。
    pub bucket: String,
    pub model_name: String,
    pub tokens: i64,
    pub quota: i64,
    pub calls: i64,
}

/// /api/monitor 行：渠道可用率。
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelAvailability {
    pub channel_key: String,
    pub days: u32,
    pub total: i64,
    pub ok_count: i64,
    pub availability: Option<f64>,
    pub avg_latency_ms: Option<f64>,
}

/// 后端列表端点的统一包装 `{"items":[...]}`。
#[derive(Debug, Default, serde::Deserialize)]
struct Items<T> {
    #[serde(default)]
    items: Vec<T>,
}

fn iso_utc_now() -> String {
    let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
    now.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// 窗口起点 RFC3339：今天→24h, 本周→7d, 本月→30d, 今年→365d。
pub fn window_start(timeframe: &str) -> String {
    use chrono::Duration;
    let now: chrono::DateTime<chrono::Utc> = chrono::Utc::now();
    let days: i64 = match timeframe {
        "今天" => 1,
        "本周" => 7,
        "本月" => 30,
        _ => 365,
    };
    (now - Duration::days(days))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

async fn get_json<T: serde::de::DeserializeOwned + Default>(path: String) -> ApiResult<T> {
    let client = ApiClient::shared().clone();
    client.get(&path).await
}

/// 真实调用: GET /api/log/top?by=user|model&start=&end=&limit=10
pub async fn top_usage_api(by: &str, start: &str, limit: u32) -> ApiResult<Vec<UsageTopRow>> {
    Ok(
        get_json::<Items<UsageTopRow>>(format!("/api/log/top?by={by}&start={start}&limit={limit}"))
            .await?
            .items,
    )
}

/// 真实调用: GET /api/log/trend?granularity=hour|day|month&start=&end=
pub async fn trend_api(granularity: &str, start: &str) -> ApiResult<Vec<UsageTrendRow>> {
    let r = get_json::<Items<UsageTrendRow>>(format!(
        "/api/log/trend?granularity={granularity}&start={start}&end={}",
        iso_utc_now()
    ))
    .await?;
    Ok(r.items)
}

/// 真实调用: GET /api/monitor?days=42 — 渠道可用率（健康度区块数据源）。
pub async fn monitor_api(days: u32) -> ApiResult<Vec<ChannelAvailability>> {
    Ok(
        get_json::<Items<ChannelAvailability>>(format!("/api/monitor?days={days}"))
            .await?
            .items,
    )
}

/// 真实调用: GET /api/channel — 渠道名映射（monitor 只回 key）。
pub async fn list_channels_api(
    client: &ApiClient,
) -> ApiResult<Vec<contract::api::admin::ChannelDto>> {
    let r = client
        .get::<Items<contract::api::admin::ChannelDto>>("/api/channel")
        .await?;
    Ok(r.items)
}

/// 前端趋势桶：label + 总量 + 与 `model_order` 对齐的堆叠序列（原始 tokens）。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrendBucketFE {
    pub label: String,
    pub show_label: bool,
    pub total: f64,
    pub per_model: Vec<f64>,
}

/// Y 轴封顶值: step = ⌈(max/4) 的最高位⌉, 返回值 = 4 × step。
///
/// 例: max=209.5M → raw=52.4M → 最高位取整 60M → 轴顶 240M (刻度 60/120/180/240);
///     max=128M → raw=32M → 40M → 轴顶 160M (刻度 40/80/120/160)。
/// 最高柱因此恒低于轴顶, 虚线刻度始终是不带零头的整齐数 (参照 new-api 的 nice ticks)。
pub fn nice_axis_max(m: f64) -> f64 {
    if m <= 0.0 {
        return 4.0; // 空窗兜底: 单位轴
    }
    let raw = m / 4.0;
    let mag = 10f64.powf(raw.log10().floor());
    // -εpsilon: 让恰好整位的 raw (如 5.0) 不被浮点误差顶到 6, 保留恰好满格的情况
    let digit = (raw / mag - 1e-9).ceil();
    digit * mag * 4.0
}

/// 把服务端 trend 行（桶×模型）pivot 成连续桶序列 + 全局模型序。
///
/// 桶粒度随 timeframe：今天→小时(24 桶)，本周/本月→天，今年→月。空桶补零，
/// 保证直方图槽位连续。模型按窗口总量降序取前 10。
pub fn pivot_trend(rows: Vec<UsageTrendRow>, timeframe: &str) -> (Vec<TrendBucketFE>, Vec<String>) {
    use chrono::{DateTime, Duration, Utc};
    use std::collections::HashMap;

    let step = Duration::hours(1);
    let (n, unit) = match timeframe {
        "今天" => (24usize, "hour"),
        "本周" => (7, "day"),
        "本月" => (30, "day"),
        _ => (12, "month"),
    };
    let step = match unit {
        "day" => Duration::days(1),
        "month" => Duration::days(30),
        _ => step,
    };

    // 请求窗口起点 = 与 trend_api 同口径的 window_start（UTC）
    let start = DateTime::parse_from_rfc3339(&window_start(timeframe))
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now() - step * n as i32);

    // 桶标签与 show_label
    let fmt_label = |t: DateTime<Utc>| match unit {
        "hour" => format!("{}:00", t.format("%H")),
        "day" => t.format("%m-%d").to_string(),
        _ => t.format("%Y-%m").to_string(),
    };
    let show_label = |i: usize, n: usize| match n {
        24 => i.is_multiple_of(4),
        30 => i.is_multiple_of(5),
        _ => true,
    };

    // 服务端行 → (bucket_idx, model) → tokens
    let mut by_bucket_model: HashMap<(usize, String), i64> = HashMap::new();
    for r in &rows {
        let t = match DateTime::parse_from_rfc3339(&r.bucket) {
            Ok(t) => t.with_timezone(&Utc),
            Err(_) => continue,
        };
        // 服务端桶与本地步长对齐：按窗口起点取整偏移
        let idx = ((t - start).num_seconds() / step.num_seconds()).max(0) as usize;
        if idx < n {
            *by_bucket_model
                .entry((idx, r.model_name.clone()))
                .or_default() += r.tokens;
        }
    }

    // 模型按总量降序取前 10
    let mut model_tot: HashMap<String, i64> = HashMap::new();
    for ((_, m), v) in by_bucket_model.iter() {
        *model_tot.entry(m.clone()).or_default() += v;
    }
    let mut order: Vec<(String, i64)> = model_tot.into_iter().collect();
    order.sort_by_key(|(_, v)| -*v);
    order.truncate(10);
    let model_order: Vec<String> = order.into_iter().map(|(m, _)| m).collect();

    let mut buckets = Vec::with_capacity(n);
    for i in 0..n {
        let bt = start + step * i as i32;
        let mut per_model = vec![0.0f64; model_order.len()];
        let mut total = 0.0f64;
        for (j, name) in model_order.iter().enumerate() {
            if let Some(v) = by_bucket_model.get(&(i, name.clone())) {
                per_model[j] = *v as f64;
                total += *v as f64;
            }
        }
        buckets.push(TrendBucketFE {
            label: fmt_label(bt),
            show_label: show_label(i, n),
            total,
            per_model,
        });
    }
    (buckets, model_order)
}
