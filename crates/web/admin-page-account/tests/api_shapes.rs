//! account page 集成测试:api.rs 薄壳的数据形状不变量。
//! mock 换成真后端时,本测试保证薄壳签名与数据自洽性不变。

use admin_page_account::api;

#[test]
fn key_stats_pairs_are_complete() {
    let stats = api::fetch_key_stats();
    assert!(!stats.is_empty(), "统计卡非空");
    for (value, label) in stats {
        assert!(!value.is_empty(), "统计卡值非空");
        assert!(!label.is_empty(), "统计卡标签非空");
    }
}

#[test]
fn keys_have_unique_ids() {
    let keys = api::fetch_keys();
    assert!(!keys.is_empty(), "至少一个密钥");
    let mut ids: Vec<_> = keys.iter().map(|k| k.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), keys.len(), "密钥 id 唯一");
}

#[test]
fn logs_timestamps_sorted_or_zero() {
    let logs = api::fetch_logs();
    assert!(!logs.is_empty(), "至少一条日志");
    // mock 允许乱序 (面板会排序), 但 timestamp 必须 > 0 (1970 后)
    for (i, log) in logs.iter().enumerate() {
        assert!(log.timestamp > 0, "日志 {i} timestamp 非零");
        assert!(!log.model.is_empty(), "日志 {i} 模型名非空");
    }
}

#[test]
fn log_models_contain_all_label() {
    let models = api::fetch_log_models();
    assert!(!models.is_empty());
    // 首项是 "全部" (不过滤) — api.rs 文档约定
    assert_eq!(models[0], "全部", "log_models 首项 = 全部");
}

#[test]
fn invite_link_starts_with_scheme() {
    let link = api::fetch_invite_link();
    assert!(link.starts_with("http"), "邀请链接 {link} 以 http 开头");
}

#[test]
fn wallet_balance_is_display_string() {
    let wallet = api::fetch_wallet();
    // mock 的 balance 是展示字符串("¥1,234.56"),非空即可
    assert!(!wallet.balance.is_empty(), "钱包余额非空");
    assert!(!wallet.currency.is_empty(), "币种非空");
}
