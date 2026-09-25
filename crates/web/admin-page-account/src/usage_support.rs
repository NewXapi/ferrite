//! 面板呈现层纯工具函数 (时间窗换算 / 数字与额度格式化 / UA 归纳)。
//! 无 UI 依赖, 供 `usage_logs`、`sessions` 面板与集成测试共用。

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

/// RFC3339 → 本地 "YYYY-MM-dd HH:mm" 展示 (分钟精度, 含年份)。
/// 会话卡「最后活跃 / 到期」用: 到期可能跨年, 必须带年份才能判断剩余有效期;
/// 解析失败原样返回 (与 [`fmt_time`] 行为一致, 保证脏数据不静默丢失)。
pub fn fmt_time_minute(rfc3339: &str) -> String {
    DateTime::parse_from_rfc3339(rfc3339)
        .map(|t| t.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|_| rfc3339.to_string())
}

/// 归纳 User-Agent 为「浏览器 · OS」短标签 (如 "Chrome · Windows"), 供会话卡展示。
///
/// 完整 UA 仍然可用 (由调用方放 `title` 属性悬停可见), 这里只做一眼可读的归纳:
/// - 浏览器: 按 UA 中特征 token 识别 Chrome / Edge / Firefox / Safari,
///   顺序即优先级 (Edge 同时含 Chrome token, 必须先判 Edge);
/// - OS: 识别 Windows / macOS / Linux / Android / iOS;
/// - 浏览器与 OS 都识别不出 (空串 / 乱串 / 纯爬虫 UA) → 回退 "未知设备"。
///
/// 注意: 这不是完整的 UA 解析器, 只覆盖主流浏览器特征; 未匹配的 UA 一律
/// 归为 "其他浏览器" / "未知系统" 而不是猜一个品牌名。
///
/// 语言口径 (刻意的中英混排): 品牌名 (Chrome / Edge / Windows / macOS 等)
/// 保留英文原文不译, 识别失败的兜底文案 (「其他浏览器」「未知系统」「未知设备」)
/// 用中文 —— 与整个 UI 的中文文案语言一致。
pub fn summarize_ua(ua: &str) -> String {
    // 统一按原串大小写做包含匹配 (UA 品牌 token 自带大小写, 直接小写化比对)
    let u = ua.to_ascii_lowercase();

    let browser = if u.contains("edg/") || u.contains("edga") || u.contains("edgios") {
        "Edge"
    } else if u.contains("firefox") || u.contains("fxios") {
        "Firefox"
    } else if u.contains("chrome") || u.contains("crios") {
        "Chrome"
    } else if u.contains("safari") {
        "Safari"
    } else {
        "其他浏览器"
    };

    let os = if u.contains("windows") {
        "Windows"
    } else if u.contains("android") {
        "Android"
    } else if u.contains("iphone") || u.contains("ipad") {
        "iOS"
    } else if u.contains("mac os") || u.contains("macintosh") {
        "macOS"
    } else if u.contains("linux") || u.contains("x11") {
        "Linux"
    } else {
        "未知系统"
    };

    // 双双识别不出 = 完全无法归纳 (空串/乱串), 回退统一占位
    if browser == "其他浏览器" && os == "未知系统" {
        return "未知设备".to_string();
    }
    format!("{browser} · {os}")
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

/// 编辑密钥弹窗的 `<input type="date">` 值 (本地时区 "YYYY-MM-DD") → UTC RFC3339。
///
/// 时区语义: 用户所选日期按 **UTC 当天最后一秒** (`T23:59:59Z`) 过期 ——
/// 即「该日期一整天 (以 UTC 计) 结束后才失效」。本地时区早于 UTC (如 UTC+8)
/// 时实际失效时刻落在所选日期次日清晨, 宁可偏晚不偏早, 避免把用户明选的
/// 当天提前杀掉。
///
/// 返回 None 的情况 (调用方应视为「不发 expires_at 字段 = 保持不变」):
/// - 输入为空串 (date input 未选值);
/// - 非法输入 (date input 理论上只产 `YYYY-MM-DD`, 这里兜底脏数据)。
pub fn date_input_to_rfc3339(date_str: &str) -> Option<String> {
    let d = chrono::NaiveDate::parse_from_str(date_str.trim(), "%Y-%m-%d").ok()?;
    let dt = d.and_hms_opt(23, 59, 59)?;
    Some(
        chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    )
}

/// RFC3339 → `<input type="date">` 需要的 UTC 日期段 ("YYYY-MM-DD")。
///
/// 编辑弹窗 prefill 用: 与 [`date_input_to_rfc3339`] 构成同一 UTC 口径的
/// 回程对应 —— 去程「所选日期 → UTC 当天 23:59:59Z」, 回程「UTC 时间戳 →
/// UTC 日期段」。两端统一按 UTC 才能保证不漂移: 若回程换算成本地日期
/// (UTC+8 会 +1 天), prefill 就比用户当初选的日期晚一天, 每次保存都
/// 静默后移。过期语义 =「所选日期当日 (UTC) 结束后失效」, 与弹窗文案一致。
/// 解析失败或空串返回空串 (date input 显示为未选值)。
pub fn rfc3339_to_date_input(rfc3339: &str) -> String {
    DateTime::parse_from_rfc3339(rfc3339)
        .map(|t| t.with_timezone(&Utc).format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

/// 长标识 (UUID / 密钥) 的短显: 前 4 + 「…」+ 后 4。
///
/// 长度 ≤ 12 时原样返回 (短串截了反而难认, "前4…后4" 比原文还长);
/// 完整值由调用方挂 `title` 悬停展示。按 char 计数, 多字节字符安全。
/// 测试覆盖见 tests/keys_display.rs (正常 UUID / 边界长度 / 多字节 / 空串)。
pub fn short_key(id: &str) -> String {
    let chars: Vec<char> = id.chars().collect();
    if chars.len() <= 12 {
        return id.to_string();
    }
    let head: String = chars.iter().take(4).collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}…{tail}")
}

/// 用量百分比 0..=100, 进度条宽度用。
///
/// 口径与 admin-page-users 的 `used_pct` (data.rs) 一致: quota <= 0
/// (未设限额 / 无配额) 一律 0, 不产生除零或负数; 超用 clamp 到 100。
/// admin-page-users 是跨 crate 参照 (只能看不能引), 这里按同口径实现,
/// 测试覆盖见 tests/keys_display.rs 的 quota=0 边界用例。
pub fn used_pct(quota: i64, used_quota: i64) -> u32 {
    if quota <= 0 {
        return 0;
    }
    ((used_quota as f64 / quota as f64) * 100.0)
        .round()
        .min(100.0) as u32
}
