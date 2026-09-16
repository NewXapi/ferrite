//! sessions 面板呈现层纯函数的不变量: UA 归纳 (summarize_ua) 与
//! 分钟精度时间格式化 (fmt_time_minute)。两者都无 UI 依赖, 直接单测。

use admin_page_account::usage_support::{fmt_time_minute, summarize_ua};

// ---- summarize_ua: 主流浏览器 + OS 组合 ----

#[test]
fn summarize_ua_chrome_on_windows() {
    // 典型桌面 Chrome UA (Chrome token + Windows token)
    let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";
    assert_eq!(summarize_ua(ua), "Chrome · Windows");
}

#[test]
fn summarize_ua_safari_on_macos() {
    // Safari 无 Chrome token, 走 Safari 分支; macOS 用 "Mac OS X" token 匹配
    let ua = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15";
    assert_eq!(summarize_ua(ua), "Safari · macOS");
}

#[test]
fn summarize_ua_firefox_on_linux() {
    let ua = "Mozilla/5.0 (X11; Linux x86_64; rv:127.0) Gecko/20100101 Firefox/127.0";
    assert_eq!(summarize_ua(ua), "Firefox · Linux");
}

#[test]
fn summarize_ua_chrome_on_android() {
    // Android Chrome 用 Mobile Safari + Chrome token, OS 走 Android 分支
    let ua = "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Mobile Safari/537.36";
    assert_eq!(summarize_ua(ua), "Chrome · Android");
}

#[test]
fn summarize_ua_edge_takes_priority_over_chrome() {
    // Edge UA 同时含 Chrome token, 必须先判 Edge 才不会误报 Chrome
    let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36 Edg/126.0.0.0";
    assert_eq!(summarize_ua(ua), "Edge · Windows");
}

#[test]
fn summarize_ua_ios_safari() {
    // iPhone Safari: 浏览器走 Safari, OS 走 iPhone 分支
    let ua = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_4 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Mobile/15E148 Safari/604.1";
    assert_eq!(summarize_ua(ua), "Safari · iOS");
}

// ---- summarize_ua: 回退行为 ----

#[test]
fn summarize_ua_empty_or_garbage_falls_back() {
    // 空串与无法识别的乱串都归纳不出任何品牌 → 回退 "未知设备"
    assert_eq!(summarize_ua(""), "未知设备");
    assert_eq!(summarize_ua("乱七八糟的字符串"), "未知设备");
    // 只识别出一半 (有 OS 无浏览器) 仍给出已知部分, 不算未知
    assert_eq!(
        summarize_ua("Mozilla/5.0 (Windows NT 10.0)"),
        "其他浏览器 · Windows"
    );
}

// ---- fmt_time_minute: 正常输入与回退 ----

#[test]
fn fmt_time_minute_parses_rfc3339_to_local_minute_precision() {
    // 合法 RFC3339 (带小数秒) → 本地时区 "YYYY-MM-dd HH:mm";
    // 只断言日期段与时区无关的部分, 避免测试机时区影响结果
    let ok = fmt_time_minute("2026-09-16T03:28:26.123Z");
    assert!(
        ok.starts_with("2026-09-16 ") && ok.len() == "2026-09-16 HH:MM".len(),
        "应输出 'YYYY-MM-dd HH:mm' 分钟精度, 实际: {ok}"
    );
}

#[test]
fn fmt_time_minute_invalid_input_passes_through() {
    // 非法输入原样返回: 与 fmt_time/fmt_time_full 行为一致,
    // 保证后端异常数据在 UI 上诚实可见, 而不是静默变空串
    assert_eq!(fmt_time_minute("not-a-date"), "not-a-date");
    assert_eq!(fmt_time_minute(""), "");
}
