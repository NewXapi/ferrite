//! 总览 tab 共享层:文案常量、统计卡视图类型与趋势悬浮卡数据形状。
//!
//! 第二轮重构把 rsx 里散落的中文硬编码收敛为 `pub const`(i18n 目标):
//! 页面 / 区块 / 组件三处复用同一份字面量,改文案只改一处,且不会被
//! 「同一句文案两种写法」分叉。本文件零逻辑、零状态。

// ---- 统计区 ----

/// 统计区标题(编号段 3 区头)。
pub const SEC_STATS: &str = "总览统计";
/// 统计区加载中占位文案。
pub const STATS_LOADING: &str = "正在加载统计…";

/// 统计卡标签:总用户(同时是 StatCard 的 `data-testid`)。
pub const LBL_USERS: &str = "总用户";
/// 统计卡标签:启用渠道。
pub const LBL_CHANNELS: &str = "启用渠道";
/// 统计卡标签:令牌。
pub const LBL_TOKENS: &str = "令牌";
/// 统计卡标签:分组。
pub const LBL_GROUPS: &str = "分组";
/// 统计卡标签:今日请求(带 sparkline 的两张卡之一)。
pub const LBL_REQUESTS_TODAY: &str = "今日请求";
/// 统计卡标签:今日额度(带 sparkline 的两张卡之一)。
pub const LBL_QUOTA_TODAY: &str = "今日额度";
/// 第 7 张卡「额度余量」的标签与 `data-testid`。
pub const LBL_QUOTA_REMAINING: &str = "额度余量";
/// 额度卡 runway 行的 `data-testid`。
pub const TESTID_QUOTA_RUNWAY: &str = "额度余量可用天数";
/// runway 态:今日无消耗(灰点,不做健康判断)。
pub const RUNWAY_NO_USAGE: &str = "无近期消耗";
/// runway 态:额度已耗尽(红点红字)。
pub const RUNWAY_EXHAUSTED: &str = "已耗尽";
/// 额度卡口径小字。
pub const QUOTA_FOOTNOTE: &str = "启用用户余额合计 ÷ 今日消耗";

/// 单张统计卡的视图:页面从 `DashboardSummaryDto` 派生后传入组件。
///
/// - `sparkline` 为 `Some` 时卡底渲染 12 点迷你面积线,空序列走等高占位;
///   仅「今日请求 / 今日额度」两张卡带序列。
/// - `gradient_id` 是同页多张 sparkline 各自的 SVG 渐变 id,避免 `url(#id)` 串线。
#[derive(Clone, PartialEq)]
pub struct StatCardView {
    pub value: String,
    pub label: &'static str,
    pub sparkline: Option<Vec<f64>>,
    pub gradient_id: &'static str,
}

/// 额度余量卡视图:剩余总额度 + 今日消耗(runway 分母)。
#[derive(Clone, PartialEq)]
pub struct QuotaView {
    /// 剩余总额度(折 $ 展示)。
    pub remaining: i64,
    /// 今日消耗(runway = remaining / today)。
    pub today: i64,
}

// ---- Top10 双榜 ----

/// Top10 模型卡标题。
pub const TOP_MODELS_TITLE: &str = "消耗前十模型";
/// Top10 用户卡标题。
pub const TOP_USERS_TITLE: &str = "消耗前十用户";
/// Top10 模型卡合计单位小字。
pub const TOP_MODELS_UNIT: &str = "tokens 合计";
/// Top10 用户卡合计单位小字。
pub const TOP_USERS_UNIT: &str = "$ 合计";
/// Top10 两榜空态文案。
pub const TOP_EMPTY: &str = "该时间窗内暂无调用";
/// Top10 模型卡合计块 `data-testid`。
pub const TESTID_TOP_MODELS_TOTAL: &str = "top-models-total";
/// Top10 用户卡合计块 `data-testid`。
pub const TESTID_TOP_USERS_TOTAL: &str = "top-users-total";
/// Top10 两榜脚注:份额与增长率的取数口径。
pub const TOP_FOOTNOTE: &str = "份额为该行占前 10 名合计的比例 · 增长环比上一等长窗口 tokens";

// ---- 用量趋势面板 ----

/// 趋势面板标题(编号段 1)。
pub const SEC_TREND: &str = "用量趋势";
/// 趋势面板右上角合计单位小字。
pub const TREND_UNIT: &str = "tokens(合计)";
/// 趋势拉取失败标题。
pub const TREND_ERR: &str = "加载趋势失败";
/// 趋势加载中文案。
pub const TREND_LOADING: &str = "正在加载趋势…";
/// 趋势空窗主文案。
pub const TREND_EMPTY: &str = "该时间窗内暂无调用数据";
/// 趋势空窗副文案(引导发起真实调用)。
pub const TREND_EMPTY_HINT: &str = "发起一次 /v1 调用后这里会展示真实用量";
/// 悬浮卡整列模式的合计行标签。
pub const TREND_TIP_TOTAL: &str = "Total";
/// 右栏数据位:峰值桶。
pub const TREND_PEAK: &str = "峰值桶";
/// 右栏数据位:平均每桶。
pub const TREND_AVG: &str = "平均每桶";
/// 右栏数据位:平均每桶副文案。
pub const TREND_AVG_SUB: &str = "均值";
/// 右栏数据位:活跃模型数。
pub const TREND_MODELS: &str = "活跃模型";
/// 右栏数据位:活跃模型副文案。
pub const TREND_MODELS_SUB: &str = "窗口内有调用";
/// 右栏数据位:区间总量。
pub const TREND_RANGE_TOTAL: &str = "区间总量";
/// 右栏数据位:区间总量副文案。
pub const TREND_RANGE_SUB: &str = "tokens";
/// 右栏图例标题。
pub const TREND_TOP5: &str = "主力模型 Top5";

/// 趋势图悬浮卡:整列分解 / 单色块详情。
///
/// 由直方图在 `onmouseenter` / `onmousemove` 里构造并写回页面持有的 signal,
/// 再由 [`super::tooltip::TrendTipCard`] 消费渲染 —— 悬浮卡本体用 `fixed`
/// 定位,不随滚动裁剪,故状态提到面板层而非柱子内部。
#[derive(Clone, PartialEq)]
pub enum TrendTip {
    /// x, y, 桶标签, 明细(名称, 颜色, 值), 桶总量
    Column(f64, f64, String, Vec<(String, &'static str, f64)>, f64),
    /// x, y, 桶标签, 模型名, 颜色, 值
    Segment(f64, f64, String, String, &'static str, f64),
}

// ---- 渠道健康面板 ----

/// 渠道健康卡标题。
pub const SEC_HEALTH: &str = "渠道健康 (近 7 天)";
/// 渠道健康加载中文案。
pub const HEALTH_LOADING: &str = "正在加载渠道健康…";
/// 渠道健康空态主文案。
pub const HEALTH_EMPTY: &str = "暂无探活数据";
/// 渠道健康空态副文案。
pub const HEALTH_EMPTY_HINT: &str =
    "monitor_history 为空 —— 渠道探活开始产生记录后这里会展示真实可用率";
/// 拉取失败的中性占位文案(与统计区同口径:不上失败原因与重试)。
pub const NEUTRAL_NO_DATA: &str = "暂无数据";
/// 汇总卡:受监控渠道。
pub const HEALTH_MONITORED: &str = "受监控渠道";
/// 汇总卡:平均可用率。
pub const HEALTH_AVAIL: &str = "平均可用率(7天)";
/// 汇总卡:探活总数。
pub const HEALTH_PROBES: &str = "探活总数(7天)";
/// 汇总卡:平均延迟。
pub const HEALTH_LATENCY: &str = "平均延迟";
/// 渠道名缺失时的兜底前缀(后接 key 前 8 位)。
pub const CHANNEL_FALLBACK: &str = "渠道";

// ---- 近 24 小时错误面板 ----

/// 错误卡标题。
pub const SEC_ERRORS: &str = "近 24 小时错误";
/// 错误卡头合计小字。
pub const ERRORS_TOTAL_LABEL: &str = "错误合计";
/// 错误面板加载中文案。
pub const ERRORS_LOADING: &str = "正在加载错误统计…";
/// 错误面板空态主文案。
pub const ERRORS_EMPTY: &str = "近 24 小时无错误记录";
/// 错误面板空态副文案。
pub const ERRORS_EMPTY_HINT: &str = "渠道调用开始产生错误流水后，这里会按模型聚合展示";

/// 无数据占位符(大数字与可用率的诚实降级:不亮假 0)。
pub const DASH: &str = "—";
