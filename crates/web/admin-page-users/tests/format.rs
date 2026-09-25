//! 展示格式化助手(从 src/data.rs 内联测试迁出)。

use admin_page_users::format::{
    cny_to_quota, fmt_cny, fmt_created_date, fmt_num, short_key, used_pct,
};

#[test]
fn quota_and_pct_math() {
    assert_eq!(fmt_cny(500_000), "¥1.00");
    assert_eq!(cny_to_quota(50.0), 25_000_000);
    assert_eq!(fmt_num(1_234_567), "1,234,567");
    assert_eq!(fmt_num(42), "42");
    // 满额与零额度边界
    assert_eq!(used_pct(1_000, 990), 99); // 接近满额
    assert_eq!(used_pct(1_000, 0), 0); // 零用量
    assert_eq!(used_pct(0, 100), 0); // 非正额度直接记 0
    assert_eq!(used_pct(100, 150), 100); // 超额封顶 100
}

#[test]
fn created_date_takes_date_part_only() {
    // 后端 RFC3339:日期为主,时分秒不进视觉主区
    assert_eq!(
        fmt_created_date("2026-09-11T09:48:45.676011Z"),
        "2026-09-11"
    );
    // 空格分隔同样截前 10 位
    assert_eq!(fmt_created_date("2026-09-11 09:48:45"), "2026-09-11");
    // 已是日期 → 原样
    assert_eq!(fmt_created_date("2026-09-11"), "2026-09-11");
    // 无法识别 → 原样返回,不冒充
    assert_eq!(fmt_created_date(""), "");
    assert_eq!(fmt_created_date("nonsense"), "nonsense");
    assert_eq!(fmt_created_date("  2026-09-11T00:00:00Z  "), "2026-09-11");
}

#[test]
fn short_key_truncates_uuid() {
    // 36 字符 UUID → 前 8 …后 4
    assert_eq!(
        short_key("00000000-0000-0000-0000-000000000001"),
        "00000000…0001"
    );
    assert_eq!(
        short_key("4129f446-4606-4693-90fc-d88abe2d0d07"),
        "4129f446…0d07"
    );
    // 短 key 原样
    assert_eq!(short_key("short"), "short");
    assert_eq!(short_key(""), "");
}
