//! EditKeyModal (编辑密钥) 的 wire 形状与日期换算不变量。
//!
//! UI 的 DOM 行为测不了 (无浏览器), 这里只锁两类纯逻辑:
//! 1. `UpdateTokenRequest` 的 serde 序列化形状 —— 编辑弹窗「留空 = 不发字段」
//!    依赖契约层的 `skip_serializing_if`, 字段名 camelCase 逐字对齐后端
//!    `UpdateTokenRequest` (admin-catalog tokens.rs, rename_all = "camelCase");
//! 2. date input ↔ RFC3339 换算纯函数 (usage_support) 的时区语义与非法回退。

use admin_page_account::usage_support::{date_input_to_rfc3339, rfc3339_to_date_input};
use contract::api::token::UpdateTokenRequest;

#[test]
fn update_token_request_all_default_omits_group_and_expires_at() {
    // 编辑弹窗「留空 = 保持不变」的实现方式: 分组/过期输入为空时根本不发字段,
    // 后端 (svc.update 逐字段 if let Some) 收到缺省即不改。若契约层丢了
    // skip_serializing_if, 序列化会出现 "group": null —— 后端把它反序列化成
    // 内层 None = 跟随用户组, 语义静默漂移, 必须在这里拦下。
    let req = UpdateTokenRequest {
        name: Some("改名".into()),
        ..Default::default()
    };
    let v = serde_json::to_value(&req).expect("UpdateTokenRequest 必须可序列化");
    let obj = v.as_object().expect("请求体必须是 JSON object");
    assert!(
        !obj.contains_key("group"),
        "全缺省请求不得出现 group 字段 (skip_serializing_if 失效)"
    );
    assert!(
        !obj.contains_key("expiresAt"),
        "全缺省请求不得出现 expiresAt 字段"
    );
    assert!(
        !obj.contains_key("quota") && !obj.contains_key("unlimitedQuota"),
        "全缺省请求不得出现 quota / unlimitedQuota 字段"
    );
    assert_eq!(obj.len(), 1, "只应有 name 一个字段");
    assert_eq!(v["name"], "改名");
}

#[test]
fn update_token_request_uses_camel_case_wire_fields() {
    // 后端 UpdateTokenRequest (admin-catalog tokens.rs) 是
    // #[serde(rename_all = "camelCase")]: wire 字段名逐字为 unlimitedQuota /
    // expiresAt; snake_case 会被 serde 静默丢弃 (default → None = 不改)。
    let req = UpdateTokenRequest {
        group: Some("vip".into()),
        quota: Some(500_000),
        unlimited_quota: Some(false),
        expires_at: Some("2026-12-31T23:59:59Z".into()),
        ..Default::default()
    };
    let v = serde_json::to_value(&req).expect("UpdateTokenRequest 必须可序列化");
    assert_eq!(
        v,
        serde_json::json!({
            "group": "vip",
            "quota": 500_000,
            "unlimitedQuota": false,
            "expiresAt": "2026-12-31T23:59:59Z",
        }),
        "带值请求的 wire 形状必须逐字 camelCase"
    );
}

#[test]
fn update_token_request_unlimited_combines_with_quota() {
    // 编辑弹窗总是成对提交 Some(quota) + Some(unlimitedQuota):
    // 勾选「无限额度」时也带 quota 数值 (后端以 unlimitedQuota=true 为准,
    // quota 只是占位, 但两字段必须在场, 保证取消勾选后 quota 有值可回)。
    let req = UpdateTokenRequest {
        quota: Some(0),
        unlimited_quota: Some(true),
        ..Default::default()
    };
    let v = serde_json::to_value(&req).expect("UpdateTokenRequest 必须可序列化");
    assert_eq!(v["unlimitedQuota"], true, "unlimitedQuota 必须在场为 true");
    assert_eq!(v["quota"], 0, "quota 数值必须在场 (可为 0 占位)");
    assert!(
        !v.as_object().unwrap().contains_key("group"),
        "未改的分组仍不发"
    );
}

#[test]
fn date_input_to_rfc3339_is_utc_end_of_day() {
    // 时区语义: 所选日期按 UTC 当天最后一秒过期, 便于只看日期时判定「已过期」。
    assert_eq!(
        date_input_to_rfc3339("2026-12-31").as_deref(),
        Some("2026-12-31T23:59:59Z"),
        "date input → UTC 当天末尾 RFC3339 (Z 结尾)"
    );
    // 路径参数兼容: 前后空白由调用方 trim, 函数内部再兜一次
    assert_eq!(
        date_input_to_rfc3339(" 2026-01-02 ").as_deref(),
        Some("2026-01-02T23:59:59Z"),
        "容忍输入两端空白"
    );
}

#[test]
fn date_input_to_rfc3339_falls_back_to_none_on_bad_input() {
    // 非法输入回退 None → 调用方不发 expires_at 字段 (= 保持不变), 不猜测日期。
    for bad in ["", "  ", "31-12-2026", "2026/12/31", "2026-13-01", "abcd"] {
        assert_eq!(
            date_input_to_rfc3339(bad),
            None,
            "非法日期输入 {bad:?} 必须回退 None"
        );
    }
}

#[test]
fn rfc3339_to_date_input_takes_local_date_segment() {
    // prefill 方向: RFC3339 → 本地日期段; 用带时区偏移的输入验证换算发生
    // (00:30Z 在 UTC+8 本地是 08:30, 同一天, 日期段不变; 换 -30s 后仍同日)。
    // 直接断言形状为 YYYY-MM-DD 且与本地解析结果一致, 不写死具体时区。
    let rfc = "2026-03-01T16:30:00Z";
    let got = rfc3339_to_date_input(rfc);
    assert_eq!(
        got.len(),
        10,
        "date input 值必须是 10 位 YYYY-MM-DD, got {got:?}"
    );
    let local = chrono::DateTime::parse_from_rfc3339(rfc)
        .unwrap()
        .with_timezone(&chrono::Local)
        .format("%Y-%m-%d")
        .to_string();
    assert_eq!(got, local, "必须换算成本地时区日期段");
}

#[test]
fn rfc3339_to_date_input_empty_on_bad_input() {
    // 非法/空输入回退空串 → date input 显示未选值, 编辑语义 = 不发字段。
    for bad in ["", "not-a-date", "2026-13-99T00:00:00Z"] {
        assert_eq!(rfc3339_to_date_input(bad), "", "非法输入 {bad:?} 回退空串");
    }
}
