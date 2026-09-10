//! 用量面板的纯工具函数 (时间窗换算 / 数字与额度格式化)。
//! 无 UI 依赖, 供 `usage_logs` 面板与集成测试共用。

/// 内部额度单位 → 美元换算基数 (后端口径: 500_000 = $1)。
pub const QUOTA_PER_USD: f64 = 500_000.0;

/// 时间范围标签 (SegmentedCapsule 显示文案 = range_bounds 的匹配键)。
pub const RANGE_TODAY: &str = "今天";
pub const RANGE_7D: &str = "7天";
pub const RANGE_30D: &str = "30天";

use chrono::{DateTime, Duration, Local, SecondsFormat, Utc};

/// 时间范围标签 → (start, end) RFC3339 (UTC, Z 结尾, 无 `+` 避免 URL 转义)。
pub fn range_bounds(label: &str) -> (String, String) {
    let end = Utc::now();
    let days = match label {
        RANGE_TODAY => 1,
        RANGE_7D => 7,
        _ => 30,
    };
    let start = end - Duration::days(days);
    (
        start.to_rfc3339_opts(SecondsFormat::Secs, true),
        end.to_rfc3339_opts(SecondsFormat::Secs, true),
    )
}

/// RFC3339 → 本地 "MM-dd HH:mm" 展示; 解析失败原样返回。
pub fn fmt_time(rfc3339: &str) -> String {
    DateTime::parse_from_rfc3339(rfc3339)
        .map(|t| t.with_timezone(&Local).format("%m-%d %H:%M").to_string())
        .unwrap_or_else(|_| rfc3339.to_string())
}

/// RFC3339 → 本地 "YYYY-MM-dd HH:mm:ss" (详情弹窗用)。
pub fn fmt_time_full(rfc3339: &str) -> String {
    DateTime::parse_from_rfc3339(rfc3339)
        .map(|t| {
            t.with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|_| rfc3339.to_string())
}

/// 千分位格式化。
pub fn fmt_num(n: i64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// 内部额度单位 → 估算美元展示 (500_000 ≈ $1)。
pub fn fmt_quota(quota: i64) -> String {
    format!("${:.4}", quota as f64 / QUOTA_PER_USD)
}
