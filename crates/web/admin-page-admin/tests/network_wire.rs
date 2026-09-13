//! NetworkPanel 数据供给层的纯函数 / DTO 形状测试。
//!
//! 覆盖:
//! - `channel_models`:渠道 `models` JSONB 展开规则(字符串 / alias 对象 / 混合 /
//!   非法形状),与 admin snapshot `expand_models_json` 对外名提取一致;
//! - `GraphView::from_dtos`:三组 DTO → 拓扑快照的投影形状;
//! - `edges_of`:与后端快照展开规则一致的边推导(groups × models 笛卡尔积,
//!   无同名别名时该模型悬空,不产生边);
//! - 空数据形状:全空 DTO 列表 → 全空 GraphView + 空边集。
//!
//! 不依赖网络 / PG,直接构造 contract DTO 断言。

use admin_page_admin::api::ModelView;
use admin_page_admin::{GraphView, NodeKey, channel_models, edges_of};
use contract::api::admin::{ChannelDto, GroupDto};
use serde_json::json;

fn group(name: &str) -> GroupDto {
    GroupDto {
        key: name.to_string(),
        name: name.to_string(),
        ratio: 1.0,
        model_whitelist: json!([]),
        remark: String::new(),
        status: 1,
        created_at: String::new(),
        updated_at: String::new(),
    }
}

fn channel(name: &str, models: serde_json::Value, groups: Vec<&str>) -> ChannelDto {
    ChannelDto {
        key: name.to_string(),
        name: name.to_string(),
        channel_type: "openai".into(),
        base_url: "https://example.invalid".into(),
        key_count: 1,
        keys: None,
        models,
        groups: groups.into_iter().map(String::from).collect(),
        priority: 0,
        weight: 1,
        status: 1,
        test_model: None,
        remark: String::new(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}

fn model(name: &str) -> ModelView {
    ModelView {
        name: name.to_string(),
    }
}

/// 字符串数组 → 原样展开。
#[test]
fn channel_models_string_array() {
    let models = json!(["gpt-4o", "gpt-5"]);
    assert_eq!(
        channel_models(&models),
        vec!["gpt-4o".to_string(), "gpt-5".to_string()]
    );
}

/// 对象数组取 `alias` 字段(对外名)。
#[test]
fn channel_models_alias_objects() {
    let models = json!([{"alias": "gpt-4o", "upstream": "gpt-4o-2024-05"}]);
    assert_eq!(channel_models(&models), vec!["gpt-4o".to_string()]);
}

/// 混合 + 非法条目静默跳过,不 panic。
#[test]
fn channel_models_mixed_and_invalid() {
    let models = json!(["ok", {"upstream": "no-alias"}, 42]);
    assert_eq!(channel_models(&models), vec!["ok".to_string()]);
}

/// 非数组(Null / 对象)→ 空列表。
#[test]
fn channel_models_non_array() {
    assert_eq!(channel_models(&json!(null)), Vec::<String>::new());
    assert_eq!(channel_models(&json!({"a": 1})), Vec::<String>::new());
}

/// from_dtos:三组 DTO 的投影形状(dispatch 保留渠道序号,channel_groups 对齐)。
#[test]
fn from_dtos_shapes() {
    let groups = [group("default"), group("vip")];
    let models = [model("gpt-4o"), model("gpt-5")];
    let channels = [
        channel("ch-a", json!(["gpt-4o"]), vec!["default", "vip"]),
        channel("ch-b", json!([{"alias": "gpt-5"}]), vec!["vip"]),
    ];
    let view = GraphView::from_dtos(&groups, &models, &channels);
    assert_eq!(view.groups, vec!["default".to_string(), "vip".to_string()]);
    assert_eq!(
        view.aliases,
        vec!["gpt-4o".to_string(), "gpt-5".to_string()]
    );
    assert_eq!(view.channels, vec!["ch-a".to_string(), "ch-b".to_string()]);
    assert_eq!(
        view.dispatch,
        vec![(0, "gpt-4o".to_string()), (1, "gpt-5".to_string())]
    );
    assert_eq!(
        view.channel_groups,
        vec![
            vec!["default".to_string(), "vip".to_string()],
            vec!["vip".to_string()]
        ]
    );
}

/// 全空 DTO → 全空快照。
#[test]
fn from_dtos_empty() {
    let view = GraphView::from_dtos(&[], &[], &[]);
    assert!(view.groups.is_empty());
    assert!(view.aliases.is_empty());
    assert!(view.channels.is_empty());
    assert!(view.dispatch.is_empty());
    assert!(view.channel_groups.is_empty());
}

/// 边推导:渠道 A 服务两个分组、一个模型 →
/// Group(default)→Mapping(gpt-4o)、Group(vip)→Mapping(gpt-4o)、Mapping→Dispatch(0);
/// 渠道 B 的模型无同名别名 → 只有 Mapping→Dispatch,没有 分组→别名 边。
#[test]
fn edges_of_cartesian_and_hanging() {
    let groups = [group("default"), group("vip")];
    let models = [model("gpt-4o")]; // 只有 gpt-4o,gpt-5 无别名
    let channels = [
        channel("ch-a", json!(["gpt-4o"]), vec!["default", "vip"]),
        channel("ch-b", json!(["gpt-5"]), vec!["vip"]),
    ];
    let view = GraphView::from_dtos(&groups, &models, &channels);
    let edges = edges_of(&view);
    // 渠道 A:gpt-4o 命中别名 0
    assert!(edges.contains(&(NodeKey::Group(0), NodeKey::Mapping(0))));
    assert!(edges.contains(&(NodeKey::Group(1), NodeKey::Mapping(0))));
    assert!(edges.contains(&(NodeKey::Mapping(0), NodeKey::Dispatch(0))));
    // 渠道 B:gpt-5 无同名别名 → 没有任何 Mapping(1)… 边,
    // 也没有指向 Dispatch(1) 的边(别名→调度模型边只在命中别名时生成)
    assert!(!edges.iter().any(|(u, _)| *u == NodeKey::Mapping(1)));
    assert!(!edges.iter().any(|(_, l)| *l == NodeKey::Dispatch(1)));
    // 渠道未服务的分组没有边:渠道 B 只服务 vip,但 vip 边来自渠道 A 的笛卡尔积,
    // 这里验证 Group(vip)→Mapping(0) 存在(来自 A),而非 B。
    assert!(edges.contains(&(NodeKey::Group(1), NodeKey::Mapping(0))));
}

/// 渠道 groups 为空 → 不产生 分组→别名 边,只有 别名→调度模型 边。
#[test]
fn edges_of_no_groups() {
    let groups = [group("default")];
    let models = [model("gpt-4o")];
    let channels = [channel("ch-a", json!(["gpt-4o"]), vec![])];
    let view = GraphView::from_dtos(&groups, &models, &channels);
    let edges = edges_of(&view);
    assert!(
        !edges
            .iter()
            .any(|(u, l)| *u == NodeKey::Group(0) && *l == NodeKey::Mapping(0))
    );
    assert!(edges.contains(&(NodeKey::Mapping(0), NodeKey::Dispatch(0))));
}

/// 全空快照 → 无边。
#[test]
fn edges_of_empty_view() {
    let view = GraphView::from_dtos(&[], &[], &[]);
    assert!(edges_of(&view).is_empty());
}
