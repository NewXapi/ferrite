//! 用户页的展示格式化助手。数据本身来自 `api`。
//!
//! 归属注记(spec 理念 2):`used_pct` / `short_key` 与 admin-page-account 的
//! `usage_support` 同口径,属跨页语义——阶段 1 上提
//! `ui-components/components/progress/`;其余为本页独有展示格式化。
// DONE: format.rs 是纯函数模块(非组件)——已移出 components/ 到 src 根级 in=demo by=agent
use crate::api;

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

/// 分组标签:分组列表来自后端,无法用静态切片返回,故返回 String。
pub fn group_label(group: &str) -> String {
    group.to_string()
}

pub fn role_label(role: u16) -> &'static str {
    api::fetch_roles()
        .iter()
        .find(|(_, v)| *v == role)
        .map(|(l, _)| *l)
        .unwrap_or("普通用户")
}

/// 用量百分比,0..=100
pub fn used_pct(quota: i64, used_quota: i64) -> u32 {
    if quota <= 0 {
        return 0;
    }
    ((used_quota as f64 / quota as f64) * 100.0)
        .round()
        .min(100.0) as u32
}

/// 后端时间戳的「日期为主」展示:截取 `YYYY-MM-DD`(RFC3339 / 空格分隔
/// 均取前 10 位;解析失败原样返回)。
///
/// 完整时刻(含时分秒)不进视觉主区,hover 时经原生 `title` 提示,
/// 避免 WASM 侧引入时区格式化依赖。
pub fn fmt_created_date(created_at: &str) -> String {
    let t = created_at.trim();
    let is_date_like = t.len() >= 10 && t.as_bytes()[4] == b'-' && t.as_bytes()[7] == b'-';
    if is_date_like {
        // get(..10) 边界安全:即便落库时刻含多字节字符也不会 panic
        // (chrono RFC3339 保证 ASCII,这是防御性兜底)
        t.get(..10).unwrap_or(t).to_string()
    } else {
        t.to_string()
    }
}

/// UUID key 的截断展示:`前 8 位…后 4 位`(短于 13 位原样返回)。
///
/// 完整 key 不进视觉主区(一行排不下),hover 经原生 `title` 提示全值。
/// key 是纯 ASCII,直接按字节切片省一次 `Vec<char>` 分配;非 ASCII
/// (理论不会出现)回落字符路径,避免切到多字节字符中间。
pub fn short_key(key: &str) -> String {
    let k = key.trim();
    if k.is_ascii() {
        if k.len() <= 13 {
            return k.to_string();
        }
        return format!("{}…{}", &k[..8], &k[k.len() - 4..]);
    }
    let chars: Vec<char> = k.chars().collect();
    if chars.len() <= 13 {
        return k.to_string();
    }
    let head: String = chars[..8].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}…{tail}")
}
