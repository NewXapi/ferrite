//! 上一等长窗:窗口长度口径 + 上一窗起点推导 + 指定 `[start, end)` 的区间取数。
//!
//! 为什么独立成文件:`api.rs` 的 `top_usage_api` 把 `end` 固定缺省 now,表达不了
//! `[上一窗起点, 当前窗起点)` 这种闭开区间,而 api.rs 是冻结区不可改;名次变动
//! 必须拿「上一窗」榜单做对比,故在此补一层薄封装。

use chrono::{DateTime, Utc};

use crate::api::UsageTopRow;
use client::{ApiClient, ApiResult};

/// 窗口长度(天):与 `crate::api::window_start` 同口径(今天=1 / 本周=7 /
/// 本月=30 / 未知兜底=365)。两边任一漂移都会让「上一窗」错位,故在此显式注释。
fn window_days(timeframe: &str) -> i64 {
    match timeframe {
        "今天" => 1,
        "本周" => 7,
        "本月" => 30,
        _ => 365,
    }
}

/// 上一等长窗起点(纯函数,给定 `now` 便于测试):当前窗起点是 `now - 窗长`,
/// 上一窗再往前一个窗长,即 `now - 2×窗长`。格式与 `api::window_start`
/// 一致(RFC3339 UTC,秒级)。
/// 注意:与 `window_start` 各自独立算 `now`,两次 `Utc::now()` 之间若有漂移,
/// 上一窗与当前窗的分界会有亚秒级误差 —— 榜单聚合按整天切窗,该误差不影响名次。
pub fn previous_window_start_from(now: DateTime<Utc>, timeframe: &str) -> String {
    use chrono::Duration;
    (now - Duration::days(window_days(timeframe) * 2))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

/// 当前时刻的上一等长窗起点(薄包装,组件直呼)。
pub fn previous_window_start(timeframe: &str) -> String {
    previous_window_start_from(Utc::now(), timeframe)
}

/// `/api/log/top` 响应的 `{"items":[...]}` 剥壳(与 api.rs 的 `Items<T>` 同形;
/// 那边是私有结构,冻结区不可复用,此处本地声明)。
#[derive(Default, serde::Deserialize)]
struct TopItems<T> {
    #[serde(default)]
    items: Vec<T>,
}

/// 真实调用: GET /api/log/top?by=&start=&end=&limit= — 指定 `[start, end)` 窗口
/// 的消耗聚合(与 [`crate::api::top_usage_api`] 同端点同 DTO,差异仅在显式传
/// `end`:名次变动的「上一窗」是 `[prev_start, cur_start)`,end 不能落 now)。
/// 错误情况:401/403(未登录或非管理员)、网络失败,均走 [`ApiResult`]。
pub async fn top_usage_between(
    by: &str,
    start: &str,
    end: &str,
    limit: u32,
) -> ApiResult<Vec<UsageTopRow>> {
    let client = ApiClient::shared().clone();
    let r: TopItems<UsageTopRow> = client
        .get(&format!(
            "/api/log/top?by={by}&start={start}&end={end}&limit={limit}"
        ))
        .await?;
    Ok(r.items)
}
