//! 验证 usage 域契约 DTO 与后端 admin-observe 真实 wire 形状兼容。
//!
//! 后端为权威：`LogView` / `LogQuery` / `UsageStat` 均带
//! `#[serde(rename_all = "camelCase")]`，时间以 RFC3339 字符串出现在 wire 上。
//! 契约 [`contract::api::usage`] 按同一形状定义（时间用 `String` 承载，
//! 不引 chrono）。本测试用 camelCase 的 wire 样例 JSON 锁死该兼容性：
//! 解码断言字段类型与语义，序列化断言 query 参数名。

use contract::api::usage::{
    UsageDailyStatDto, UsageLogDto, UsageLogPage, UsageLogQuery, UsageStatDto,
};

/// `GET /api/log` 响应体 `{"items": [LogView...], "total": n}` 的 wire 样例。
/// camelCase、`userKey` 为 UUID 字符串、`createdAt` 为 RFC3339。
fn real_log_page_body() -> &'static str {
    r#"{"items":[{"id":42,"logType":2,"userKey":"d9334bb6-1652-4e00-ade8-75fe5d5fbc45","username":"probe_user","tokenName":"work","channelName":"chan-a","modelName":"gpt-4o","promptTokens":128,"completionTokens":64,"quota":95000,"useTimeMs":812,"isStream":true,"ip":"127.0.0.1","requestId":"req-0001","createdAt":"2026-09-08T17:55:28.579140Z"}],"total":1}"#
}

#[test]
fn usage_log_dto_parses_backend_wire_shape() {
    let page: UsageLogPage = serde_json::from_str(real_log_page_body())
        .expect("后端真实 log 页响应必须能被 UsageLogPage 解析");

    assert_eq!(page.total, 1, "total 为 i64 命中总数");
    // 显式标注元素类型: items 必须恰好是 UsageLogDto。
    let row: &UsageLogDto = &page.items[0];
    assert_eq!(row.id, 42, "id 为数据库 BIGSERIAL 主键 (i64)");
    assert_eq!(row.log_type, 2, "logType=2 即 consume (i16)");
    assert_eq!(row.user_key, "d9334bb6-1652-4e00-ade8-75fe5d5fbc45");
    assert_eq!(row.username, "probe_user");
    assert_eq!(row.token_name, "work");
    assert_eq!(row.channel_name, "chan-a");
    assert_eq!(row.model_name, "gpt-4o");
    assert_eq!(row.prompt_tokens, 128, "promptTokens 为 i32");
    assert_eq!(row.completion_tokens, 64, "completionTokens 为 i32");
    assert_eq!(row.quota, 95_000, "quota 为 i64 内部额度单位 (500_000≈$1)");
    assert_eq!(row.use_time_ms, 812, "useTimeMs 为 i32 毫秒");
    assert!(row.is_stream, "isStream 布尔位");
    assert_eq!(row.ip, "127.0.0.1");
    assert_eq!(row.request_id, "req-0001");
    assert_eq!(
        row.created_at, "2026-09-08T17:55:28.579140Z",
        "createdAt 为 RFC3339 字符串"
    );
}

#[test]
fn usage_stat_dto_parses_wire_and_defaults_to_zero() {
    // 后端 UsageStat 的 wire 形状：四个 i64，camelCase 下均为单单词键。
    let stat: UsageStatDto =
        serde_json::from_str(r#"{"quota":5000000,"requests":10,"rpm":3,"tpm":2048}"#)
            .expect("后端 stat 响应必须能被 UsageStatDto 解析");
    assert_eq!(stat.quota, 5_000_000);
    assert_eq!(stat.requests, 10);
    assert_eq!(stat.rpm, 3, "rpm 为近 60s 请求数");
    assert_eq!(stat.tpm, 2048, "tpm 为近 60s token 数");

    let empty = UsageStatDto::default();
    assert_eq!(
        empty,
        UsageStatDto {
            quota: 0,
            requests: 0,
            rpm: 0,
            tpm: 0
        },
        "Default 全零"
    );
}

#[test]
fn usage_daily_stat_dto_parses_wire_shape() {
    let row: UsageDailyStatDto = serde_json::from_str(
        r#"{"date":"2026-09-08","requests":12,"tokens":3456,"quota":7000000}"#,
    )
    .expect("按天聚合 wire 行必须能被 UsageDailyStatDto 解析");
    assert_eq!(row.date, "2026-09-08", "date 为 YYYY-MM-DD 分组键");
    assert_eq!(row.requests, 12);
    assert_eq!(row.tokens, 3456, "tokens = prompt + completion");
    assert_eq!(row.quota, 7_000_000, "quota 为内部额度单位");
}

#[test]
fn usage_log_query_serializes_camel_case_param_names() {
    // 全量填充后序列化，键名必须与后端 LogQuery 的 camelCase query 参数一一对应。
    let q = UsageLogQuery {
        log_type: Some(2),
        username: Some("probe_user".into()),
        token_name: Some("work".into()),
        model_name: Some("gpt-4o".into()),
        start: Some("2026-09-08T00:00:00Z".into()),
        end: Some("2026-09-09T00:00:00Z".into()),
        page: Some(1),
        size: Some(20),
    };
    let json: serde_json::Value = serde_json::to_value(&q).expect("query 必须可序列化");
    assert_eq!(
        json,
        serde_json::json!({
            "logType": 2,
            "username": "probe_user",
            "tokenName": "work",
            "modelName": "gpt-4o",
            "start": "2026-09-08T00:00:00Z",
            "end": "2026-09-09T00:00:00Z",
            "page": 1,
            "size": 20
        }),
        "序列化键名须为后端认识的 camelCase 参数名"
    );
}

#[test]
fn usage_log_query_omits_unset_filters() {
    // 未设置的过滤条件不产生键 — 等价于 URL query 中不带该参数,
    // 后端 Option 解析为 None 走默认值 (page=1, size=20)。
    let q = UsageLogQuery {
        page: Some(2),
        size: Some(50),
        ..Default::default()
    };
    let json: serde_json::Value = serde_json::to_value(&q).expect("query 必须可序列化");
    assert_eq!(
        json,
        serde_json::json!({ "page": 2, "size": 50 }),
        "None 字段被省略, 不发出 null 参数"
    );
}

#[test]
fn usage_log_query_deserializes_camel_case_subset() {
    // 后端 query 参数子集（URL 解码后的键值形态）必须能解回 UsageLogQuery，
    // 未出现的键保持 None —— 等价于 URL 中缺省该参数。
    let json = serde_json::json!({
        "logType": 2, "modelName": "gpt-4o", "start": "2026-09-08T00:00:00Z", "page": 1, "size": 20
    });
    let q: UsageLogQuery =
        serde_json::from_value(json).expect("camelCase 键须能解回 UsageLogQuery");
    assert_eq!(
        q,
        UsageLogQuery {
            log_type: Some(2),
            model_name: Some("gpt-4o".into()),
            start: Some("2026-09-08T00:00:00Z".into()),
            page: Some(1),
            size: Some(20),
            ..Default::default()
        }
    );
    assert!(q.username.is_none() && q.token_name.is_none() && q.end.is_none());
}
