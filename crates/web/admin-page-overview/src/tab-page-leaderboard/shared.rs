//! 排行榜 tab 共享层:文案常量(i18n)与 `slug` 纯函数。
//!
//! 本文件零逻辑、零状态;纯判定与聚合留在 `movers` / `vendors` / `prev_window`。

/// 上升 / 下跌卡各取前几名(两卡口径一致)。
pub const MOVERS_LIMIT: usize = 6;

// ---- 真实用量榜区 ----

/// 真实用量榜标题。
pub const USAGE_TITLE: &str = "真实用量榜";
/// 副标题前缀(后接窗口内有调用的模型数)。
pub const USAGE_SUBTITLE_HEAD: &str = "后端消费日志聚合 · 窗口内 ";
/// 副标题后缀。
pub const USAGE_SUBTITLE_TAIL: &str = " 个模型有调用";
// ---- 演示实力榜区 ----

/// 模型实力榜(演示)标题与 `aria-label`。
pub const DEMO_TITLE: &str = "模型实力榜";
/// 收录数徽标前缀(后接模型数)。
pub const DEMO_COUNT_HEAD: &str = "共收录 ";
/// 收录数徽标后缀。
pub const DEMO_COUNT_TAIL: &str = " 款主流模型";

/// 时间窗胶囊的 `data-testid` 前缀(实际为 `{prefix}-{档位}`)。
pub const TESTID_TIMEFRAME_PREFIX: Option<&'static str> = Some("leaderboard-timeframe");
/// 用量榜拉取失败文案。
pub const USAGE_ERR: &str = "加载排行数据失败";
/// 失败卡的重试按钮文案(本面板保留重试,与总览三面板的中性占位不同)。
pub const BTN_RETRY: &str = "重试";
/// 用量榜加载中文案。
pub const USAGE_LOADING: &str = "正在加载排行数据…";
/// 用量榜空窗主文案。
pub const USAGE_EMPTY: &str = "该时间窗内暂无调用数据";
/// 用量榜空窗副文案。
pub const USAGE_EMPTY_HINT: &str = "发起一次 /v1 调用后这里会展示真实用量排行";

// ---- 三张口径榜 ----

/// Token 口径卡标题。
pub const RANK_TOKENS_TITLE: &str = "Token 消耗 Top";
/// Token 口径卡副标题。
pub const RANK_TOKENS_SUBTITLE: &str = "窗口内 prompt + completion tokens 合计";
/// 调用次数字口径卡标题。
pub const RANK_CALLS_TITLE: &str = "调用次数 Top";
/// 调用次数字口径卡副标题。
pub const RANK_CALLS_SUBTITLE: &str = "窗口内消费请求数";
/// 费用口径卡标题。
pub const RANK_QUOTA_TITLE: &str = "费用消耗 Top";
/// 费用口径卡副标题(500000 额度 = $1)。
pub const RANK_QUOTA_SUBTITLE: &str = "窗口内计费额度(500000 = $1)";
/// 三张榜共用脚注:增长率与份额的取数口径。
pub const RANK_FOOTNOTE: &str = "增长率为 tokens 环比(上一等长窗);份额为行值占当榜合计";

// ---- 升降速双卡 ----

/// 上升卡标题。
pub const MOVERS_TITLE: &str = "上升最快";
/// 上升卡副标题。
pub const MOVERS_SUBTITLE: &str = "tokens 名次较上一等长窗上升(取前 6)";
/// 下跌卡标题。
pub const DROPPERS_TITLE: &str = "下跌最快";
/// 下跌卡副标题。
pub const DROPPERS_SUBTITLE: &str = "tokens 名次较上一等长窗下跌(取前 6)";
/// 双卡空态文案(两窗对比无显著变动)。
pub const MOVERS_EMPTY: &str = "当前窗口无显著变动";
/// 双卡失败文案前缀(后接错误串;诚实报错,不假装「无变动」)。
pub const MOVERS_ERR_PREFIX: &str = "名次变动加载失败:";
/// 上升卡条色(emerald)。
pub const MOVERS_BAR_COLOR: &str = "#34d399";
/// 下跌卡条色(rose)。
pub const DROPPERS_BAR_COLOR: &str = "#fb7185";

// ---- 厂商份额卡 ----

/// 厂商份额卡标题。
pub const VENDOR_TITLE: &str = "厂商份额";
/// 厂商份额卡副标题。
pub const VENDOR_SUBTITLE: &str = "窗口内各厂商 Token 消耗占比";
/// 厂商份额卡脚注(口径声明:厂商按模型名前缀推断)。
pub const VENDOR_FOOTNOTE: &str = "厂商按模型名前缀推断";

// ---- 演示图表卡 ----

/// 用量分布卡标题。
pub const CHART_DIST_TITLE: &str = "模型用量与成本占比";
/// 用量分布卡副标题。
pub const CHART_DIST_SUBTITLE: &str = "Token 消耗分布与费用占比";
/// 用量分布行小字中段(前接日均请求千数,后接上下文千数)。
pub const CHART_REQ_MID: &str = "K 次请求 · 上下文 ";
/// SLA 性能矩阵卡标题。
pub const CHART_SLA_TITLE: &str = "网关响应与 SLA 性能矩阵";
/// SLA 性能矩阵卡副标题。
pub const CHART_SLA_SUBTITLE: &str = "端到端 P50 延迟、吞吐与高可用";
/// SLA 徽标(演示值)。
pub const CHART_SLA_BADGE: &str = "SLA 99.94%";
/// 指标:P50 均值延迟。
pub const CHART_P50: &str = "P50 均值延迟";
/// 指标:P90 尾部延迟。
pub const CHART_P90: &str = "P90 尾部延迟";
/// 指标:峰值吞吐 TPS。
pub const CHART_TPS: &str = "峰值吞吐 TPS";
/// 指标:平均成功率。
pub const CHART_SUCCESS: &str = "平均成功率";
/// 分组配额卡标题。
pub const CHART_GROUP_TITLE: &str = "分组配额与倍率分布";
/// 分组配额卡副标题。
pub const CHART_GROUP_SUBTITLE: &str = "租户路由分组及倍率消耗";
/// 分组配额卡徽标(演示值)。
pub const CHART_GROUP_BADGE: &str = "4 个活跃分组";
/// 分组卡底:默认路由权重。
pub const CHART_ROUTE_WEIGHT: &str = "默认路由权重:";
/// 分组卡底:路由权重值。
pub const CHART_ROUTE_PRIORITY: &str = "Priority 优先";
/// 分组卡底:自动降级熔断。
pub const CHART_CIRCUIT_BREAKER: &str = "自动降级熔断:";
/// 分组卡底:熔断状态值。
pub const CHART_CIRCUIT_ON: &str = "已开启";

/// 模型/厂商名 → data-testid slug:ASCII 字母数字保留(转小写),其余折叠为
/// 单个 `-`(如 `deepseek-ai/DeepSeek-V3` → `deepseek-ai-deepseek-v3`),
/// 供 UI 验证按 name 语义定位。
pub fn slug(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}
