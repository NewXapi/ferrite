//! network.rs 写路径的纯函数测试:请求构造(以服务端 DTO 为基底、只覆盖
//! 抽屉里编辑过的字段、掩码密钥不回传)与 API DTO → EntityStore 行的映射。
//! 这些项以 `#[doc(hidden)] pub` 暴露,仅为满足"测试统一放 tests/"的
//! 项目约定;非公共 API,勿在 crate 之外使用。
//!
//! 纯函数不触 Signal,可在裸测试环境直接构造 DTO(均有 Default)。

use admin_page_admin::network::{
    channel_row_from_dto, channel_upsert_for_import, channel_upsert_from_dto, group_row_from_dto,
    group_upsert_from_dto, models_json_to_names, parse_key_lines,
};
use contract::api::admin::{ChannelDto, GroupDto};
use serde_json::json;

/// 测试基底分组 DTO:服务端现值(倍率/白名单)抽屉不可编辑。
fn base_group_dto() -> GroupDto {
    GroupDto {
        key: "grp-1".into(),
        name: "vip".into(),
        ratio: 0.8,
        model_whitelist: json!(["gpt-4o", "gpt-5"]),
        remark: "VIP 专线".into(),
        status: 1,
        ..Default::default()
    }
}

/// 测试基底渠道 DTO:调度模型/分组/权重等服务端现值,keys 是掩码回显。
fn base_channel_dto() -> ChannelDto {
    ChannelDto {
        key: "chn-7".into(),
        name: "OpenAI 官方".into(),
        channel_type: "openai".into(),
        base_url: "https://api.openai.com/v1".into(),
        key_count: 2,
        keys: Some(vec!["sk-****".into(), "sk-****2".into()]),
        models: json!(["gpt-4o", { "alias": "gpt-5x", "upstream": "raw-name" }]),
        groups: vec!["default".into(), "vip".into()],
        priority: 3,
        weight: 5,
        status: 1,
        test_model: Some("gpt-4o".into()),
        remark: "官方直连".into(),
        ..Default::default()
    }
}

// ---------- 密钥行解析 ----------

/// 多行 Key 文本按行拆分、去首尾空白、丢弃空行;空文本 → 空 vec
/// (后端约定空 = 不改密钥)。
#[test]
fn parse_key_lines_splits_trims_and_drops_blanks() {
    assert_eq!(
        parse_key_lines("sk-a\n  sk-b  \n\n\nsk-c"),
        vec!["sk-a".to_string(), "sk-b".to_string(), "sk-c".to_string()]
    );
    assert!(parse_key_lines("").is_empty());
    assert!(parse_key_lines("   \n\t\n").is_empty(), "纯空白行不算 key");
}

// ---------- models JSONB 解析 ----------

/// models 元素是字符串本身,或取 {"alias"} 字段;非数组 → 空列表。
#[test]
fn models_json_to_names_handles_strings_and_alias_objects() {
    let dto = base_channel_dto();
    assert_eq!(
        models_json_to_names(&dto.models),
        vec!["gpt-4o".to_string(), "gpt-5x".to_string()]
    );
    assert!(models_json_to_names(&json!(null)).is_empty());
    assert!(models_json_to_names(&json!("not-an-array")).is_empty());
    assert!(models_json_to_names(&json!([])).is_empty());
}

// ---------- 分组更新请求 ----------

/// 更新请求以服务端 DTO 为基底:名称/备注被覆盖,倍率与模型白名单
/// 必须原样保留——否则一次分组改名会顺手把白名单清空。
#[test]
fn group_upsert_from_dto_overrides_name_remark_keeps_server_fields() {
    let dto = base_group_dto();
    let req = group_upsert_from_dto(&dto, "  vip-pro  ", "新展示名");
    assert_eq!(req.name, "vip-pro");
    assert_eq!(req.remark, "新展示名");
    assert_eq!(req.ratio, 0.8, "倍率抽屉不可编辑,必须保留服务端现值");
    assert_eq!(
        req.model_whitelist,
        json!(["gpt-4o", "gpt-5"]),
        "白名单必须保留服务端现值,不得置空"
    );
}

// ---------- 渠道更新请求 ----------

/// 渠道更新请求覆盖名称/URL/密钥;调度模型、分组、权重、测速模型、
/// 备注等服务端现值原样保留(抽屉里不可编辑)。
#[test]
fn channel_upsert_from_dto_overrides_edited_fields_keeps_server_fields() {
    let dto = base_channel_dto();
    let req = channel_upsert_from_dto(&dto, "  新名字  ", "https://mirror.example/v1", "");
    assert_eq!(req.name, "新名字");
    assert_eq!(req.base_url, "https://mirror.example/v1");
    assert_eq!(req.channel_type, "openai");
    assert_eq!(req.models, dto.models, "调度模型不得被更新请求清掉");
    assert_eq!(req.groups, vec!["default".to_string(), "vip".to_string()]);
    assert_eq!(req.priority, 3);
    assert_eq!(req.weight, 5);
    assert_eq!(req.test_model, Some("gpt-4o".into()));
    assert_eq!(req.remark, "官方直连");
}

/// 密钥安全:只携带用户本次输入的明文 key 行;留空 = 空 vec(后端
/// 约定不变),绝不把列表响应里的掩码值(dto.keys)回传给服务端。
#[test]
fn channel_upsert_from_dto_never_echoes_masked_keys() {
    let dto = base_channel_dto();
    // 掩码值进 → 不出:请求 keys 与 dto.keys 无交集
    let req = channel_upsert_from_dto(&dto, "n", "u", "");
    assert!(req.keys.is_empty(), "留空 = 不改密钥,不得携带任何值");
    let req = channel_upsert_from_dto(&dto, "n", "u", "sk-new-1\n sk-new-2 \n");
    assert_eq!(
        req.keys,
        vec!["sk-new-1".to_string(), "sk-new-2".to_string()],
        "只携带用户输入的明文 key"
    );
    assert!(
        !req.keys.iter().any(|k| k.contains('*')),
        "掩码值绝不能被回传"
    );
}

// ---------- 导入面板创建请求 ----------

/// 导入请求:名称留空回落「新渠道」,类型 openai、分组 default、
/// 模型留空(待设置页拉取)、无权重/优先级;Key 按行解析。
#[test]
fn channel_upsert_for_import_defaults() {
    let req = channel_upsert_for_import("", " https://one.example/v1 ", "sk-1\nsk-2");
    assert_eq!(req.name, "新渠道");
    assert_eq!(req.channel_type, "openai");
    assert_eq!(req.base_url, "https://one.example/v1");
    assert_eq!(req.groups, vec!["default".to_string()]);
    assert_eq!(req.models, json!([]));
    assert_eq!(req.priority, 0);
    assert_eq!(req.weight, 0);
    assert_eq!(req.test_model, None);
    assert_eq!(req.keys, vec!["sk-1".to_string(), "sk-2".to_string()]);

    let named = channel_upsert_for_import("  自定义渠道  ", "https://x.example", "");
    assert_eq!(named.name, "自定义渠道", "非空名称原样保留(trim 后)");
}

// ---------- API DTO → EntityStore 行 ----------

/// 分组行映射与 hydrate 一致:remark 空 → 展示名回落为标识。
#[test]
fn group_row_from_dto_maps_remark_to_display() {
    let mut dto = base_group_dto();
    let row = group_row_from_dto(&dto);
    assert_eq!(row.name, "vip");
    assert_eq!(row.display, "VIP 专线");
    assert_eq!(row.multiplier, 0.8);

    dto.remark = String::new();
    let row = group_row_from_dto(&dto);
    assert_eq!(row.display, "vip", "备注为空时展示名回落为分组标识");
}

/// 渠道行映射与 hydrate 一致:status 1→启用,其余→停用;groups 空回落
/// default;keys 永远不落地(列表响应只有掩码);dispatch/candidates
/// 来自 models 解析。
#[test]
fn channel_row_from_dto_maps_hydrate_shape() {
    let dto = base_channel_dto();
    let row = channel_row_from_dto(&dto);
    assert_eq!(row.name, "OpenAI 官方");
    assert_eq!(row.ctype, "openai");
    assert_eq!(row.url, "https://api.openai.com/v1");
    assert_eq!(row.keys, "", "掩码密钥不得写进 store");
    assert_eq!(row.status, 1, "status=1 → 启用");
    assert_eq!(row.group, "default,vip");
    assert_eq!(
        row.dispatch,
        vec!["gpt-4o".to_string(), "gpt-5x".to_string()]
    );
    assert_eq!(
        row.candidates,
        vec![("gpt-4o".to_string(), false), ("gpt-5x".to_string(), false)]
    );

    // status=2(自动停用)→ store 停用;groups 空 → 回落 default;
    // channel_type 空 → 回落 openai。
    let mut dto = base_channel_dto();
    dto.status = 2;
    dto.groups = vec![];
    dto.channel_type = String::new();
    let row = channel_row_from_dto(&dto);
    assert_eq!(row.status, 0);
    assert_eq!(row.group, "default");
    assert_eq!(row.ctype, "openai");
}
