//! 渠道页纯函数单测:filter_channels / parse_keys_input / parse_group_input。
//! 全部为无 runtime、无网络的纯函数,直接对 `crate::channels` 的
//! `pub` 函数做断言,验证 wire 前的数据形状。

use admin_page_admin::channels::{filter_channels, parse_group_input, parse_keys_input};
use contract::api::admin::ChannelDto;

fn dto(
    key: &str,
    name: &str,
    ctype: &str,
    base_url: &str,
    status: i16,
    groups: Vec<&str>,
) -> ChannelDto {
    ChannelDto {
        key: key.to_string(),
        name: name.to_string(),
        channel_type: ctype.to_string(),
        base_url: base_url.to_string(),
        key_count: 1,
        keys: None,
        models: serde_json::json!([]),
        groups: groups.into_iter().map(String::from).collect(),
        priority: 0,
        weight: 0,
        status,
        test_model: None,
        remark: String::new(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}

/// tier=0 时关键词空 → 全量返回(顺序保持)。
#[test]
fn filter_channels_all_returns_full_list_in_order() {
    let list = vec![
        dto(
            "1",
            "openai",
            "openai",
            "https://api.openai.com/v1",
            1,
            vec!["default"],
        ),
        dto(
            "2",
            "claude",
            "claude",
            "https://api.anthropic.com",
            2,
            vec!["claude"],
        ),
    ];
    let got = filter_channels(&list, "", 0);
    assert_eq!(got, list, "空关键词 + tier=0 应原样返回");
}

/// 关键词匹配:名称/类型/地址/分组任一命中即可。
#[test]
fn filter_channels_keyword_hits_any_field() {
    let list = vec![
        dto(
            "1",
            "OpenAI 官方",
            "openai",
            "https://api.openai.com/v1",
            1,
            vec!["default"],
        ),
        dto(
            "2",
            "Azure",
            "openai-compat",
            "https://east.azure.example",
            1,
            vec!["vip"],
        ),
        dto(
            "3",
            "Claude",
            "claude",
            "https://api.anthropic.com",
            1,
            vec!["claude"],
        ),
    ];
    // 名称命中(大小写不敏感):key=1 名称含 openai,key=2 类型含 openai
    assert_eq!(
        filter_channels(&list, "openai", 0).len(),
        2,
        "名称或类型命中 openai 的两条"
    );
    assert!(
        filter_channels(&list, "openai", 0)
            .iter()
            .any(|c| c.key == "1")
    );
    // 分组命中:key=3 名称/类型/分组均含 claude,key=1/2 无
    assert_eq!(
        filter_channels(&list, "claude", 0).len(),
        1,
        "claude 仅命中 key=3"
    );
    assert_eq!(filter_channels(&list, "claude", 0)[0].key, "3");
    // 地址命中
    assert_eq!(filter_channels(&list, "azure", 0).len(), 1);
    // 无命中
    assert!(filter_channels(&list, "nomatch", 0).is_empty());
}

/// tier 过滤:1=仅启用(status==1),2=仅停用(status!=1),其他=全部。
#[test]
fn filter_channels_tier_by_status() {
    let list = vec![
        dto("1", "a", "openai", "u1", 1, vec![]),
        dto("2", "b", "openai", "u2", 2, vec![]),
        dto("3", "c", "openai", "u3", 0, vec![]),
    ];
    assert_eq!(filter_channels(&list, "", 1).len(), 1);
    assert_eq!(filter_channels(&list, "", 1)[0].key, "1");
    assert_eq!(
        filter_channels(&list, "", 2).len(),
        2,
        "status!=1 都算停用/异常"
    );
    // tier=99 走默认全量分支
    assert_eq!(filter_channels(&list, "", 99).len(), 3);
}

/// 关键词 + tier 叠加:关键词先筛,再按 tier 收窄。
#[test]
fn filter_channels_keyword_and_tier_combined() {
    let list = vec![
        dto("1", "openai-on", "openai", "u1", 1, vec![]),
        dto("2", "openai-off", "openai", "u2", 2, vec![]),
        dto("3", "claude-off", "claude", "u3", 2, vec![]),
    ];
    let got = filter_channels(&list, "openai", 2);
    assert_eq!(got.len(), 1, "openai 且停用只有 key=2");
    assert_eq!(got[0].key, "2");
}

/// 关键词 trim 后为空 → 等价全量(tier 仍生效)。
#[test]
fn filter_channels_trims_whitespace_query() {
    let list = vec![dto("1", "a", "openai", "u", 1, vec![])];
    assert_eq!(filter_channels(&list, "   ", 0), list);
    let list_disabled = vec![dto("1", "a", "openai", "u", 2, vec![])];
    assert!(
        filter_channels(&list_disabled, "   ", 1).is_empty(),
        "tier=1 只留启用中"
    );
    assert!(
        !filter_channels(&list_disabled, "   ", 2).is_empty(),
        "tier=2 收停用"
    );
}

/// keys 输入:换行分隔,trim,去空行。
#[test]
fn parse_keys_input_splits_and_drops_empty() {
    assert_eq!(
        parse_keys_input("sk-a\nsk-b\n\n   \nsk-c\n"),
        vec!["sk-a", "sk-b", "sk-c"],
        "换行分隔 + trim + 去空行"
    );
    assert_eq!(parse_keys_input(""), Vec::<String>::new(), "空串→空 vec");
    assert_eq!(
        parse_keys_input("  "),
        Vec::<String>::new(),
        "纯空白→空 vec"
    );
}

/// groups 输入:逗号分隔,trim,去空项。
#[test]
fn parse_group_input_splits_commas_and_drops_empty() {
    assert_eq!(
        parse_group_input("default, claude ,  ,vip"),
        vec!["default", "claude", "vip"],
        "逗号分隔 + trim + 去空项"
    );
    assert_eq!(parse_group_input(""), Vec::<String>::new());
    // 单个无逗号
    assert_eq!(parse_group_input("default"), vec!["default"]);
}
