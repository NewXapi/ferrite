use api::dispatch::{RouteIndex, ChannelConfig, ModelRoute};

/// RouteIndex 构造和修改
#[test]
fn route_index_constructs_and_updates() {
    let idx = RouteIndex::new();
    let channels = vec![ChannelConfig {
        id: 1,
        name: "test".into(),
        base_url: "https://example.com".into(),
        channel_type: "openai".into(),
        keys: vec!["sk-test".into()],
        models: vec![ModelRoute {
            alias: "gpt-4".into(),
            upstream: "gpt-4-turbo".into(),
        }],
    }];
    idx.build_from_channels(&channels);
    let resolved = idx.resolve("gpt-4").unwrap();
    assert_eq!(resolved.channel_id, 1);
    assert_eq!(resolved.upstream_model.as_ref(), "gpt-4-turbo");
}

/// RouteIndex::resolve 不存在的 model 返回错误
#[test]
fn route_index_resolve_unknown_model_returns_error() {
    let idx = RouteIndex::new();
    let result = idx.resolve("nonexistent");
    assert!(result.is_err());
}

/// RouteIndex::list_models 返回排序后的列表
#[test]
fn route_index_list_models_sorted() {
    let idx = RouteIndex::new();
    let channels = vec![
        ChannelConfig {
            id: 1,
            name: "ch1".into(),
            base_url: "https://a.com".into(),
            channel_type: "openai".into(),
            keys: vec!["sk-1".into()],
            models: vec![ModelRoute { alias: "z-model".into(), upstream: "z".into() }],
        },
        ChannelConfig {
            id: 2,
            name: "ch2".into(),
            base_url: "https://b.com".into(),
            channel_type: "openai".into(),
            keys: vec!["sk-2".into()],
            models: vec![ModelRoute { alias: "a-model".into(), upstream: "a".into() }],
        },
    ];
    idx.build_from_channels(&channels);
    let models = idx.list_models();
    assert_eq!(models, vec!["a-model", "z-model"]);
}
