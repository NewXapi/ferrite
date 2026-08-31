//! 用户页的展示格式化助手。数据本身来自 `api`。

use crate::api::{self, User};

/// new-api 约定:500000 quota = ¥1
const QUOTA_PER_CNY: f64 = 500_000.0;

/// quota → 人民币展示,沿用账户页格式
pub fn fmt_cny(quota: i64) -> String {
    format!("¥{:.2}", quota as f64 / QUOTA_PER_CNY)
}

/// 人民币金额 → quota,充值弹窗换算展示
pub fn cny_to_quota(amount: f64) -> i64 {
    (amount * QUOTA_PER_CNY) as i64
}

/// 千分位
pub fn fmt_num(n: u32) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

pub fn group_label(group: &str) -> &'static str {
    api::fetch_groups()
        .iter()
        .find(|(_, v)| *v == group)
        .map(|(l, _)| *l)
        .unwrap_or("其他")
}

pub fn role_label(role: u16) -> &'static str {
    api::fetch_roles()
        .iter()
        .find(|(_, v)| *v == role)
        .map(|(l, _)| *l)
        .unwrap_or("普通用户")
}

/// 用量百分比,0..=100
pub fn used_pct(u: &User) -> u32 {
    if u.quota <= 0 {
        return 0;
    }
    ((u.used_quota as f64 / u.quota as f64) * 100.0)
        .round()
        .min(100.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quota_and_pct_math() {
        assert_eq!(fmt_cny(500_000), "¥1.00");
        assert_eq!(cny_to_quota(50.0), 25_000_000);
        assert_eq!(fmt_num(1_234_567), "1,234,567");
        assert_eq!(fmt_num(42), "42");
        // 满额与零额度边界
        let users = api::fetch_users();
        assert_eq!(used_pct(&users[3]), 99);
        assert_eq!(used_pct(&users[5]), 0);
    }
}
