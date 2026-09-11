//! usage_logs 工具函数的纯函数不变量 (CRG 测试缺口: range_bounds/fmt_num/fmt_quota)。

use admin_page_account::usage_support::{fmt_num, fmt_quota, fmt_time_full, range_bounds};

#[test]
fn fmt_num_thousands_separators() {
    assert_eq!(fmt_num(0), "0");
    assert_eq!(fmt_num(7), "7");
    assert_eq!(fmt_num(999), "999");
    assert_eq!(fmt_num(1000), "1,000");
    assert_eq!(fmt_num(1234567), "1,234,567");
    assert_eq!(fmt_num(-1234), "-1,234");
}

#[test]
fn fmt_quota_converts_internal_units() {
    // 后端口径: 500_000 内部单位 = $1
    assert_eq!(fmt_quota(0), "$0.0000");
    assert_eq!(fmt_quota(500_000), "$1.0000");
    assert_eq!(fmt_quota(250_000), "$0.5000");
    assert_eq!(fmt_quota(123_456), "$0.2469");
}

#[test]
fn range_bounds_cover_requested_days() {
    // start/end 为 RFC3339 (Z 结尾), end > start, 跨度约等于请求天数
    let (s, e) = range_bounds("7天");
    assert!(s.ends_with('Z'), "start 必须以 Z 结尾 (URL 中 + 会被转义)");
    assert!(e.ends_with('Z'));
    let sp: chrono::DateTime<chrono::Utc> = chrono::DateTime::parse_from_rfc3339(&s)
        .expect("start 必须是合法 RFC3339")
        .into();
    let ep: chrono::DateTime<chrono::Utc> = chrono::DateTime::parse_from_rfc3339(&e)
        .expect("end 必须是合法 RFC3339")
        .into();
    let span = ep - sp;
    assert_eq!(span.num_days(), 7);
}

#[test]
fn fmt_time_full_parses_rfc3339_or_passes_through() {
    // 合法 RFC3339 → 本地格式化 (含日期段); 非法输入原样返回, 不 panic
    let ok = fmt_time_full("2026-09-10T07:25:49.440319Z");
    assert!(ok.contains("2026-09-10"));
    assert_eq!(fmt_time_full("not-a-date"), "not-a-date");
}
