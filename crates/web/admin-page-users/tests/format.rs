//! 展示格式化助手(从 src/data.rs 内联测试迁出)。

use admin_page_users::data::{cny_to_quota, fmt_cny, fmt_num, used_pct};

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
