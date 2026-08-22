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
