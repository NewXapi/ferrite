//! Dashboard 各页的数据来源。面板只从这里取数,不认识数据是怎么来的。

use client::{ApiClient, ApiResult};
use contract::api::usage::{DashboardSummaryDto, UsageErrorStatPage, UsageLogPage, UsageStatDto};

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
    /// 上一等长窗口的同实体 tokens 合计（后端 W1 新增，口径见 contract
    /// `UsageTopRowDto::previous_tokens`）。`#[serde(default)]`：老 wire 缺该字段
    /// 时按 0 兜底，解析不失败——增长率按「上窗无数据」诚实降级。
    #[serde(default)]
    pub previous_tokens: i64,
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

/// 真实调用: GET /api/log/errors?hours=24&limit=10 — 近 N 小时错误流水按模型聚合。
///
/// 响应信封 `{"items":[{modelName,count,lastSeenAt}],"asOf"}` 就是 contract 的
/// [`UsageErrorStatPage`] 本体（items + asOf 双字段），直接整体反序列化即可，
/// 不需要再过 [`Items<T>`] 单字段剥壳。`asOf` 带 `#[serde(default)]`：
/// 旧 wire 缺该字段时落空串，调用方对 [`UsageErrorStatPage::as_of`] 做
/// [`as_of_local_time`] 解析失败即隐藏（诚实降级，不伪造时间）。
/// 错误情况:401/403(未登录或非管理员)、网络失败,均走 [`ApiResult`]。
pub async fn errors_api(hours: u32, limit: u32) -> ApiResult<UsageErrorStatPage> {
    get_json::<UsageErrorStatPage>(format!("/api/log/errors?hours={hours}&limit={limit}")).await
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

// ---- 总览统计卡的纯展示函数（无 UI 依赖，供 overview.rs 与 tests/api_shapes.rs 共用）----

/// 内部计费额度 → 美元换算基数（与后端同口径：500_000 内部单位 ≈ $1）。
pub const QUOTA_PER_USD: f64 = 500_000.0;

/// 内部计费额度 → 美元展示串（500_000 = $1，如 20_900_000 → `$41.80`）。
///
/// 万美元起切紧凑 K/M/B（`$41.8K`），防止榜卡头合计大数字与统计卡撑爆一行
/// （参照 new-api 的数字纪律：超宽切紧凑格式）；万元以内保留两位小数原值。
/// 只做展示折算，不改任何后端口径。
pub fn fmt_usd(quota: i64) -> String {
    let v = quota as f64 / QUOTA_PER_USD;
    if v.abs() >= 10_000_000_000.0 {
        format!("${:.1}B", v / 1_000_000_000.0)
    } else if v.abs() >= 10_000_000.0 {
        format!("${:.1}M", v / 1_000_000.0)
    } else if v.abs() >= 10_000.0 {
        format!("${:.1}K", v / 1_000.0)
    } else {
        format!("${v:.2}")
    }
}

/// RFC3339 数据截止时刻（`DashboardSummaryDto::as_of`）→ 本地时区 `HH:MM:SS`。
///
/// 返回 `None` 表示解析失败（后端缺字段/空串/非 RFC3339），调用方诚实降级不展示；
/// 绝不伪造时间。
pub fn as_of_local_time(as_of: &str) -> Option<String> {
    chrono::DateTime::parse_from_rfc3339(as_of).ok().map(|t| {
        t.with_timezone(&chrono::Local)
            .format("%H:%M:%S")
            .to_string()
    })
}

/// 把 hour 粒度的 trend 行按小时桶聚合，返回 `(tokens 序列, calls 序列)`。
///
/// 各 24 桶、与 `start`（RFC3339，须与拉数时传给 `/api/log/trend` 的 start 同值，
/// 避免跨小时边界重算导致的桶漂移）对齐；桶解析失败或窗口外的行丢弃。
/// `start` 本身解析失败 → 返回空序列（调用方渲染占位，诚实降级）。
pub fn hourly_sums(rows: &[UsageTrendRow], start: &str) -> (Vec<f64>, Vec<f64>) {
    use chrono::{DateTime, Utc};
    let start_t = match DateTime::parse_from_rfc3339(start) {
        Ok(t) => t.with_timezone(&Utc),
        Err(_) => return (Vec::new(), Vec::new()),
    };
    let mut tokens = vec![0.0f64; 24];
    let mut calls = vec![0.0f64; 24];
    for r in rows {
        let Ok(t) = DateTime::parse_from_rfc3339(&r.bucket) else {
            continue;
        };
        // div_euclid 而非截断除法:窗口起点之前的行(负偏移)必须落负桶被丢弃,
        // 否则截断归 0 会污染第 0 桶(单测 hourly_sums_buckets_rows_into_24_hourly_slots 钉住)。
        let idx = (t.with_timezone(&Utc) - start_t)
            .num_seconds()
            .div_euclid(3600);
        if (0..24).contains(&idx) {
            tokens[idx as usize] += r.tokens as f64;
            calls[idx as usize] += r.calls as f64;
        }
    }
    (tokens, calls)
}

/// 等距序列均分成 `buckets` 组、组内求和（24 桶重切 12 桶 = 相邻两桶合并）。
///
/// 长度恰为整数倍时逐组等分；不足时每组 1 个原值（单点 → 单桶）；
/// 空输入或 `buckets == 0` 返回空（调用方渲染等高占位）。
pub fn reslice_sum(series: &[f64], buckets: usize) -> Vec<f64> {
    if series.is_empty() || buckets == 0 {
        return Vec::new();
    }
    let per = series.len().div_ceil(buckets).max(1);
    series.chunks(per).map(|c| c.iter().sum()).collect()
}

/// min-max 归一化的 sparkline 点列：x 等距铺满 `[0, width]`，y 已翻转到屏幕坐标系
/// （0 = 顶）并上下各留 2.5px 描边余量防裁切。
///
/// 全等序列（含全零、单点）压成等高水平线（y = 0.75 × 高）——诚实呈现「无起伏」；
/// 空输入返回空（调用方渲染等高占位）。配方参照 todo/new-api-charts-leaderboard-deepdive.md §二-3。
pub fn sparkline_points(values: &[f64], width: f64, height: f64) -> Vec<(f64, f64)> {
    if values.is_empty() {
        return Vec::new();
    }
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let span = max - min;
    const PAD: f64 = 2.5;
    let usable = height - PAD * 2.0;
    let n = values.len();
    values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let x = if n == 1 {
                0.0
            } else {
                i as f64 * width / (n - 1) as f64
            };
            let y = if span <= f64::EPSILON {
                height * 0.75
            } else {
                PAD + (1.0 - (v - min) / span) * usable
            };
            (x, y)
        })
        .collect()
}

/// sparkline 的 `(line path, area path)` 的 SVG `d` 串。
///
/// line 从左到右连点；area = line + 右下角、左下角闭合（面积渐变的底边）。
/// 数据不足两点（空序列/单点）返回 `None`，调用方渲染等高占位防布局跳动。
pub fn sparkline_svg_paths(values: &[f64], width: f64, height: f64) -> Option<(String, String)> {
    let pts = sparkline_points(values, width, height);
    if pts.len() < 2 {
        return None;
    }
    let line = pts
        .iter()
        .enumerate()
        .map(|(i, (x, y))| {
            let sep = if i == 0 { "M" } else { " L" };
            format!("{sep}{x:.1} {y:.1}")
        })
        .collect::<String>();
    let area = format!("{line} L {width:.1} {height:.1} L 0 {height:.1} Z");
    Some((line, area))
}

/// Top 榜行内增长率（`previous_tokens` 环比上一等长窗口）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Growth {
    /// 上一窗有数据：升/持平的百分点数（0 表示持平）。
    Up(i64),
    /// 上一窗有数据：降的百分点数。
    Down(i64),
    /// 上一窗无该实体、本窗有消耗（新上榜）。
    New,
}

impl Growth {
    /// 展示文本：箭头编进字符串（`↑50%` / `↓50%` / `↑new`），
    /// 配合 tabular-nums 保证列对齐（参照 deepdive §二-7 GrowthText）。
    pub fn label(&self) -> String {
        match self {
            Growth::Up(n) => format!("↑{n}%"),
            Growth::Down(n) => format!("↓{n}%"),
            Growth::New => "↑new".to_string(),
        }
    }

    /// 文本色 class：升/新绿（emerald）、降红（rose），zinc 暗色卡上可读的双档。
    pub fn text_class(&self) -> &'static str {
        match self {
            Growth::Up(_) | Growth::New => "text-emerald-400",
            Growth::Down(_) => "text-rose-400",
        }
    }
}

/// 增长率三态计算（口径：本窗 tokens 对比上一等长窗口 `previous_tokens`）。
///
/// - 本窗为 0 → 不显示（`None`，含「上窗有、本窗无」的归零情形）；
/// - 上窗为 0 且本窗 > 0 → [`Growth::New`]；
/// - 其余按百分比四舍五入，升/持平 → [`Growth::Up`]，降 → [`Growth::Down`]。
pub fn growth_of(previous_tokens: i64, current_tokens: i64) -> Option<Growth> {
    if current_tokens == 0 {
        return None;
    }
    if previous_tokens == 0 {
        return Some(Growth::New);
    }
    let pct =
        ((current_tokens - previous_tokens) as f64 / previous_tokens as f64 * 100.0).round() as i64;
    Some(if pct >= 0 {
        Growth::Up(pct)
    } else {
        Growth::Down(-pct)
    })
}

/// 份额展示串：行值占当榜行值合计的比例，百分比 1 位小数。
///
/// - 合计 ≤ 0 → `None`（无榜可占比，不显示）；
/// - 正值但不足 0.1% → `<0.1%`（特判防 0.0% 误导）；
/// - 行值为 0 → `0.0%`。
pub fn share_text(value: i64, total: i64) -> Option<String> {
    if total <= 0 {
        return None;
    }
    let pct = value as f64 / total as f64 * 100.0;
    if pct > 0.0 && pct < 0.1 {
        Some("<0.1%".to_string())
    } else {
        Some(format!("{pct:.1}%"))
    }
}

/// 趋势时间窗副标题：随 timeframe 变化，与 [`window_start`] 的窗口语义一致。
///
/// 今天→24 个小时桶、本周→7 个天桶、本月→30 个天桶、今年→12 个月桶
/// （桶数与 [`pivot_trend`] 的 n 一一对应）。未知 timeframe 落今年档（同
/// [`window_start`] 的 `_` 兜底口径）。
pub fn window_caption(timeframe: &str) -> &'static str {
    match timeframe {
        "今天" => "近 24 小时 · 逐小时",
        "本周" => "近 7 天 · 逐天",
        "本月" => "近 30 天 · 逐天",
        _ => "近 12 个月 · 逐月",
    }
}

/// 悬浮卡整列分解的折叠阈值：明细超过 10 行时尾部折叠为一行「+N more」。
pub const TIP_MAX_ROWS: usize = 10;

/// 「+N more」折叠行的系列色块颜色：中性 zinc（聚合多系列，不再专属某个模型色），
/// 取 [`crate::overview`] 模型调色板的 zinc 档 #a1a1aa 同值。
pub const TIP_MORE_COLOR: &str = "#a1a1aa";

/// 趋势图整列分解悬浮卡的内容：Total 合计 + 排序/折叠后的展示行。
#[derive(Debug, Clone, PartialEq)]
pub struct TrendColumnTip {
    /// 全列合计（所有原始值之和，含被 0.01 阈值过滤的微值），渲染为顶部「Total」行。
    pub total: f64,
    /// 展示行 `(模型名, 系列色, 原始值)`：按值降序；超过 [`TIP_MAX_ROWS`] 行时
    /// 第 11 行起折叠为一行——name = `+N more`、value = 被折叠行的值合计
    /// （合计行/可见行/折叠行三段对得上账）、color = 中性 zinc [`TIP_MORE_COLOR`]。
    pub rows: Vec<(String, &'static str, f64)>,
}

/// 整列分解悬浮卡的纯整形：排序（值降序）+ Total + 超限折叠。
///
/// 输入 `values`/`names` 与桶的 `per_model`/`model_order` 同序对齐（zip 取短边），
/// `colors` 为系列色板（按原始下标取模，与直方图堆叠段同色）。值 ≤ 0.01 的微段
/// 不进明细（与原悬浮卡过滤口径一致，避免一屏零碎行）；`total` 仍含全部原始值。
/// 空输入 / 全零 → `rows` 空、`total` 0（调用方只剩 Total 行，诚实呈现空列）。
pub fn trend_column_tip(
    values: &[f64],
    names: &[String],
    colors: &[&'static str],
) -> TrendColumnTip {
    // 系列色与名称都按「原始下标 i」取(与直方图堆叠段同色,不随排序漂移);
    // names.get(i) 缺位时该行跳过(调用方契约本就是 values/names 同序对齐)。
    let mut rows: Vec<(String, &'static str, f64)> = values
        .iter()
        .enumerate()
        .filter_map(|(i, v)| {
            let name = names.get(i)?;
            (*v > 0.01).then(|| (name.clone(), colors[i % colors.len()], *v))
        })
        .collect();
    rows.sort_by(|a, z| z.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    let total: f64 = values.iter().sum();
    if rows.len() > TIP_MAX_ROWS {
        let folded: f64 = rows[TIP_MAX_ROWS..].iter().map(|(_, _, v)| *v).sum();
        let more = rows.len() - TIP_MAX_ROWS;
        rows.truncate(TIP_MAX_ROWS);
        rows.push((format!("+{more} more"), TIP_MORE_COLOR, folded));
    }
    TrendColumnTip { total, rows }
}

/// RFC3339 时刻（`UsageErrorStatDto::last_seen_at` 等）→ 本地时区 `HH:MM`。
///
/// 错误榜行内只需时:分两段，比 [`as_of_local_time`] 的 HH:MM:SS 更省宽。
/// 返回 `None` 表示解析失败（空串/非 RFC3339），调用方以 `—` 占位，不伪造时间。
pub fn last_seen_local_time(rfc3339: &str) -> Option<String> {
    chrono::DateTime::parse_from_rfc3339(rfc3339)
        .ok()
        .map(|t| t.with_timezone(&chrono::Local).format("%H:%M").to_string())
}
