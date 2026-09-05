use api::dispatch::ModelRoute;
use api::gateway::{
    CreateChannelReq, LogQuery, auth_error_type, error_response, filter_log_lines, gen_token_key,
    validate_channel,
};
use axum::http::StatusCode;

/// key 格式：sk- + 32 hex
#[test]
fn gen_token_key_format() {
    let k = gen_token_key();
    assert!(k.starts_with("sk-"));
    assert_eq!(k.len(), 3 + 32);
    assert!(k[3..].bytes().all(|b| b.is_ascii_hexdigit()));
}

/// 两次生成的 key 应不同（随机性兜检验）
#[test]
fn gen_token_key_unique() {
    assert_ne!(gen_token_key(), gen_token_key());
}

fn valid_req() -> CreateChannelReq {
    CreateChannelReq {
        name: "openai-main".into(),
        base_url: "https://api.openai.com".into(),
        channel_type: "openai".into(),
        keys: vec!["sk-upstream-key".into()],
        models: vec![ModelRoute {
            alias: "gpt-4o".into(),
            upstream: "gpt-4o".into(),
        }],
    }
}

/// 合法渠道配置通过校验
#[test]
fn validate_channel_happy() {
    assert!(validate_channel(&valid_req()).is_ok());
}

/// 非法 channel_type / 空 models / 坏 base_url 一律拒绝
#[test]
fn validate_channel_rejects_bad_input() {
    let mut bad_type = valid_req();
    bad_type.channel_type = "unknown".into();
    assert!(
        validate_channel(&bad_type)
            .unwrap_err()
            .contains("channel_type")
    );

    let mut no_models = valid_req();
    no_models.models.clear();
    assert!(validate_channel(&no_models).unwrap_err().contains("models"));

    let mut bad_url = valid_req();
    bad_url.base_url = "not a url".into();
    assert!(validate_channel(&bad_url).unwrap_err().contains("base_url"));
}

/// 空/空格 name、空 keys、含空字符串 keys、空 alias/upstream 均拒绝
#[test]
fn validate_channel_rejects_empty_fields() {
    let mut bad = valid_req();
    bad.name = "   ".into();
    assert!(validate_channel(&bad).unwrap_err().contains("name"));

    let mut bad = valid_req();
    bad.keys.clear();
    assert!(validate_channel(&bad).unwrap_err().contains("keys"));

    let mut bad = valid_req();
    bad.keys = vec!["".into()];
    assert!(validate_channel(&bad).unwrap_err().contains("keys"));

    let mut bad = valid_req();
    bad.models[0].alias = "".into();
    assert!(validate_channel(&bad).unwrap_err().contains("alias"));

    let mut bad = valid_req();
    bad.models[0].upstream = "  ".into();
    assert!(validate_channel(&bad).unwrap_err().contains("upstream"));
}

/// deny_unknown_fields：多余 typo 字段必须报错，不能静默吞掉
#[test]
fn create_token_req_deny_unknown_fields() {
    assert!(
        serde_json::from_str::<api::gateway::CreateTokenReq>(r#"{"username":"x","quota_usd":5}"#)
            .is_err()
    );
    assert!(serde_json::from_str::<api::gateway::CreateTokenReq>(r#"{"username":"x"}"#).is_ok());
}

fn valid_channel_cfg() -> api::dispatch::ChannelConfig {
    api::dispatch::ChannelConfig {
        id: 1,
        name: "openai-main".into(),
        base_url: "https://api.openai.com".into(),
        channel_type: "openai".into(),
        keys: vec!["sk-key1".into()],
        models: vec![api::dispatch::ModelRoute {
            alias: "gpt-4o".into(),
            upstream: "gpt-4o".into(),
        }],
    }
}

/// mask_key：≥9 字符显示前 8+...，短 key 不 panic
#[test]
fn mask_key_behaviour() {
    assert_eq!(api::gateway::mask_key("sk-abcdefgh123"), "sk-abcde...");
    assert_eq!(api::gateway::mask_key("short"), "***");
}

/// mask_channel_keys：每个 key 都掩码
#[test]
fn mask_channel_keys_all_masked() {
    let keys = vec!["sk-abcdefghij".to_string(), "sk-xyz".to_string()];
    let masked = api::gateway::mask_channel_keys(&keys);
    assert_eq!(masked.len(), 2);
    assert!(masked[0].ends_with("..."));
    assert!(!masked[0].contains("abcdefghij"));
}

/// merge_channel_config：None 字段不变，Some 字段覆写；changed 旗标准确
#[test]
fn merge_channel_config_partial_update() {
    use api::gateway::{UpdateChannelReq, merge_channel_config};
    let base = valid_channel_cfg();

    // 空更新：全部不变
    let req = serde_json::from_str::<UpdateChannelReq>("{}").unwrap();
    let (merged, changed) = merge_channel_config(&base, &req);
    assert!(!changed);
    assert_eq!(merged.name, base.name);

    // 只改 name
    let req = serde_json::from_str::<UpdateChannelReq>(r#"{"name":"renamed"}"#).unwrap();
    let (merged, changed) = merge_channel_config(&base, &req);
    assert!(changed);
    assert_eq!(merged.name, "renamed");
    assert_eq!(merged.base_url, base.base_url);
    assert_eq!(merged.keys, base.keys);

    // 同值不标 changed
    let req = serde_json::from_str::<UpdateChannelReq>(&format!(r#"{{"name":"{}"}}"#, base.name))
        .unwrap();
    let (merged, changed) = merge_channel_config(&base, &req);
    assert!(!changed);
    assert_eq!(merged.name, base.name);
}

/// UpdateChannelReq：多余字段 400（deny_unknown_fields）
#[test]
fn update_channel_req_deny_unknown_fields() {
    assert!(
        serde_json::from_str::<api::gateway::UpdateChannelReq>(r#"{"name":"x","typo":1}"#).is_err()
    );
}

/// merge 边界：keys=[] （非 None) 算 change，随后 validate 会拒绝（三层校验证实）
#[test]
fn merge_channel_config_empty_vec_is_change() {
    use api::gateway::{UpdateChannelReq, merge_channel_config};
    let base = valid_channel_cfg();
    let req = serde_json::from_str::<UpdateChannelReq>(r#"{"keys":[]}"#).unwrap();
    let (merged, changed) = merge_channel_config(&base, &req);
    assert!(changed);
    assert!(merged.keys.is_empty());
    // validate 链路禁止空 keys，PUT handler 会 400
    let create_req = api::gateway::CreateChannelReq {
        name: merged.name,
        base_url: merged.base_url,
        channel_type: merged.channel_type,
        keys: merged.keys,
        models: merged.models,
    };
    assert!(api::gateway::validate_channel(&create_req).is_err());

    // models 字段 merge
    let req =
        serde_json::from_str::<UpdateChannelReq>(r#"{"models":[{"alias":"m8","upstream":"m8"}]}"#)
            .unwrap();
    let (merged, changed) = merge_channel_config(&base, &req);
    assert!(changed);
    assert_eq!(merged.models[0].alias, "m8");
}

/// RechargeReq：多余 typo 字段必须报错（deny_unknown_fields）
#[test]
fn recharge_req_deny_typo() {
    assert!(
        serde_json::from_str::<api::gateway::RechargeReq>(
            r#"{"token_key":"sk-abc","amount":100,"typo":1}"#
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<api::gateway::RechargeReq>(r#"{"token_key":"sk-abc","amount":100}"#)
            .is_ok()
    );
}

// ─── 从 src/gateway.rs 迁出：错误响应形状 + 日志过滤 ───────────────────
#[test]
fn auth_error_type_maps_openai_types() {
    assert_eq!(auth_error_type(StatusCode::UNAUTHORIZED), "invalid_api_key");
    assert_eq!(auth_error_type(StatusCode::FORBIDDEN), "permission_denied");
    assert_eq!(
        auth_error_type(StatusCode::PAYMENT_REQUIRED),
        "insufficient_quota"
    );
    assert_eq!(
        auth_error_type(StatusCode::INTERNAL_SERVER_ERROR),
        "server_error"
    );
}

#[tokio::test]
async fn error_response_is_json_with_openai_shape() {
    let resp = error_response(StatusCode::BAD_REQUEST, "bad", "invalid_request_error");
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.headers()
            .get(axum::http::header::CONTENT_TYPE)
            .unwrap(),
        "application/json"
    );
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"]["message"], "bad");
    assert_eq!(v["error"]["type"], "invalid_request_error");
}

#[test]
fn filter_log_lines_filters_and_paginates() {
    // 完成事件行（有 fields.status）
    let line1 = r#"{"timestamp":"2025-01-01T00:00:00Z","fields":{"status":200,"user":"alice","model":"gpt-4"},"span":{"channel":"ch1","path":"/v1/chat/completions"}}"#;
    // 非完成事件行（无 fields.status）→ 跳过
    let line2 = r#"{"timestamp":"2025-01-01T00:01:00Z","fields":{"user":"bob"}}"#;
    // status 不匹配
    let line3 = r#"{"timestamp":"2025-01-01T00:02:00Z","fields":{"status":500,"user":"alice"}}"#;
    // 损坏行 → 跳过
    let line4 = "not json at all";

    let lines = [line1, line2, line3, line4];

    // 无过滤：1 匹配（只有 line1 有 status）
    let q = LogQuery {
        user: None,
        model: None,
        channel: None,
        path: None,
        status: None,
        since: None,
        until: None,
        limit: None,
        offset: None,
    };
    let (page, total) = filter_log_lines(&lines, &q);
    assert_eq!(total, 2); // line1 (200) + line3 (500)
    assert_eq!(page.len(), 2);

    // 按 user 过滤
    let q = LogQuery {
        user: Some("alice".into()),
        model: None,
        channel: None,
        path: None,
        status: None,
        since: None,
        until: None,
        limit: None,
        offset: None,
    };
    let (_, total) = filter_log_lines(&lines, &q);
    assert_eq!(total, 2); // line1 + line3 都是 alice

    // 按 status 过滤
    let q = LogQuery {
        user: None,
        model: None,
        channel: None,
        path: None,
        status: Some(200),
        since: None,
        until: None,
        limit: None,
        offset: None,
    };
    let (page, total) = filter_log_lines(&lines, &q);
    assert_eq!(total, 1);
    assert_eq!(page[0]["status"].as_u64(), Some(200));
    assert_eq!(page[0]["user"].as_str(), Some("alice"));
    assert_eq!(page[0]["channel"].as_str(), Some("ch1")); // 来自 span 展平

    // since 前缀比较
    let q = LogQuery {
        user: None,
        model: None,
        channel: None,
        path: None,
        status: None,
        since: Some("2025-01-01T00:01:30Z".into()),
        until: None,
        limit: None,
        offset: None,
    };
    let (_, total) = filter_log_lines(&lines, &q);
    assert_eq!(total, 1); // 只有 line3 在 00:02:00

    // 分页
    let q = LogQuery {
        user: None,
        model: None,
        channel: None,
        path: None,
        status: None,
        since: None,
        until: None,
        limit: Some(1),
        offset: Some(1),
    };
    let (page, total) = filter_log_lines(&lines, &q);
    assert_eq!(total, 2); // 全量
    assert_eq!(page.len(), 1); // 第二页只取 1 条
}
