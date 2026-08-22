use api::dispatch::ModelRoute;
use api::gateway::{CreateChannelReq, gen_token_key, validate_channel};

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
        models: vec![ModelRoute { alias: "gpt-4o".into(), upstream: "gpt-4o".into() }],
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
    assert!(validate_channel(&bad_type).unwrap_err().contains("channel_type"));

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
    assert!(serde_json::from_str::<api::gateway::CreateTokenReq>(
        r#"{"username":"x","quota_usd":5}"#
    )
    .is_err());
    assert!(serde_json::from_str::<api::gateway::CreateTokenReq>(r#"{"username":"x"}"#).is_ok());
}

fn valid_channel_cfg() -> api::dispatch::ChannelConfig {
    api::dispatch::ChannelConfig {
        id: 1,
        name: "openai-main".into(),
        base_url: "https://api.openai.com".into(),
        channel_type: "openai".into(),
        keys: vec!["sk-key1".into()],
        models: vec![api::dispatch::ModelRoute { alias: "gpt-4o".into(), upstream: "gpt-4o".into() }],
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
    use api::gateway::{merge_channel_config, UpdateChannelReq};
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
    let req = serde_json::from_str::<UpdateChannelReq>(&format!(r#"{{"name":"{}"}}"#, base.name)).unwrap();
    let (merged, changed) = merge_channel_config(&base, &req);
    assert!(!changed);
    assert_eq!(merged.name, base.name);
}

/// UpdateChannelReq：多余字段 400（deny_unknown_fields）
#[test]
fn update_channel_req_deny_unknown_fields() {
    assert!(serde_json::from_str::<api::gateway::UpdateChannelReq>(r#"{"name":"x","typo":1}"#).is_err());
}

/// merge 边界：keys=[] （非 None) 算 change，随后 validate 会拒绝（三层校验证实）
#[test]
fn merge_channel_config_empty_vec_is_change() {
    use api::gateway::{merge_channel_config, UpdateChannelReq};
    let base = valid_channel_cfg();
    let req = serde_json::from_str::<UpdateChannelReq>(r#"{"keys":[]}"#).unwrap();
    let (merged, changed) = merge_channel_config(&base, &req);
    assert!(changed);
    assert!(merged.keys.is_empty());
    // validate 链路禁止空 keys，PUT handler 会 400
    let create_req = api::gateway::CreateChannelReq {
        name: merged.name, base_url: merged.base_url, channel_type: merged.channel_type,
        keys: merged.keys, models: merged.models,
    };
    assert!(api::gateway::validate_channel(&create_req).is_err());

    // models 字段 merge
    let req = serde_json::from_str::<UpdateChannelReq>(
        r#"{"models":[{"alias":"m8","upstream":"m8"}]}"#).unwrap();
    let (merged, changed) = merge_channel_config(&base, &req);
    assert!(changed);
    assert_eq!(merged.models[0].alias, "m8");
}

/// RechargeReq：多余 typo 字段必须报错（deny_unknown_fields）
#[test]
fn recharge_req_deny_typo() {
    assert!(serde_json::from_str::<api::gateway::RechargeReq>(
        r#"{"token_key":"sk-abc","amount":100,"typo":1}"#
    )
    .is_err());
    assert!(serde_json::from_str::<api::gateway::RechargeReq>(
        r#"{"token_key":"sk-abc","amount":100}"#
    )
    .is_ok());
}
