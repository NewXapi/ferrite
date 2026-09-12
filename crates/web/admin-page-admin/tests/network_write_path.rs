//! network.rs 写路径的纯函数测试:请求构造(后端不可更新的列只读化/
//! 最小 diff、未改动的 Option 列省略、掩码密钥不回传、items 信封解码)
//! 与 API DTO → EntityStore 行的映射。
//! 这些项以 `#[doc(hidden)] pub` 暴露,仅为满足"测试统一放 tests/"的
//! 项目约定;非公共 API,勿在 crate 之外使用。
//!
//! 纯函数不触 Signal,可在裸测试环境直接构造 DTO(均有 Default)。

use admin_page_admin::network::{
    channel_row_from_dto, channel_update_body, channel_upsert_for_import, group_row_from_dto,
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

/// 分组名后端**无更新路径**(PUT /api/group/{key} 的 UPDATE 只含
/// ratio/model_whitelist/remark/status):请求体的 name 必须固定取服务端
/// 现值,抽屉里生效的编辑只有展示备注;倍率与白名单原样保留,不得置空。
#[test]
fn group_upsert_from_dto_locks_name_and_keeps_server_fields() {
    let dto = base_group_dto();
    let req = group_upsert_from_dto(&dto, "  新展示名  ");
    assert_eq!(req.name, "vip", "分组名不可编辑,请求体固定携带服务端现名");
    assert_eq!(req.remark, "新展示名", "展示备注 trim 后覆盖");
    assert_eq!(req.ratio, 0.8, "倍率抽屉不可编辑,必须保留服务端现值");
    assert_eq!(
        req.model_whitelist,
        json!(["gpt-4o", "gpt-5"]),
        "白名单必须保留服务端现值,不得置空"
    );
}

// ---------- 渠道更新请求体 ----------

/// 最小 diff 请求体:只带 name/baseUrl/testModel;密钥留空时**整体省略
/// `keys` 字段**(服务端 Option 列缺席 = 保持不变)。裸契约发 `[]` 会被
/// 解成 Some([]) 并被 validate 以 "at least one key required" 拒绝,
/// 导致不改密钥就无法保存名称/URL——此测试钉死该形状。
#[test]
fn channel_update_body_omits_keys_field_when_unchanged() {
    let dto = base_channel_dto();
    let body = channel_update_body(&dto, "  新名字  ", " https://mirror.example/v1 ", "");
    assert_eq!(body["name"], json!("新名字"), "name 可更新且 trim");
    assert_eq!(body["baseUrl"], json!("https://mirror.example/v1"));
    assert!(
        body.get("keys").is_none(),
        "未输入密钥时必须整体省略 keys 字段"
    );
    assert!(
        body.get("models").is_none(),
        "其余 Option 列省略 = COALESCE 保持现值"
    );
    assert!(body.get("groups").is_none());
    // test_model 列后端 UPDATE 没有 COALESCE,省略 = 清 NULL,必须回传现值
    assert_eq!(body["testModel"], json!("gpt-4o"));
}

/// 密钥安全与 testModel 空值往返:只携带用户输入的明文行,绝不回传
/// 掩码值;dto.test_model 为 None 时显式携带 null(与后端 NULL 现状一致)。
#[test]
fn channel_update_body_carries_typed_keys_and_explicit_null_test_model() {
    let dto = base_channel_dto();
    let body = channel_update_body(&dto, "n", "https://x.example", "sk-new-1\n sk-new-2 \n");
    assert_eq!(body["keys"], json!(["sk-new-1", "sk-new-2"]));
    let s = body.to_string();
    assert!(!s.contains("***"), "掩码值绝不能出现在请求体: {s}");

    let mut no_test = base_channel_dto();
    no_test.test_model = None;
    let body = channel_update_body(&no_test, "n", "https://x.example", "");
    assert_eq!(body["testModel"], serde_json::Value::Null);
    assert!(body.get("keys").is_none());
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

// ---------- 列表预拉取的信封形状 ----------

use admin_page_admin::network::Items;

/// 写前按名定位 key 的预拉取必须按 `{items:[...]}` 信封解码:
/// `/api/group` 返回 `{"items":[...]}`、`/api/channel` 额外带 `total`。
/// 此测试锁定形状,防止误按裸 `Vec` 解码
/// (CI smoke 实测报 "invalid type: map, expected a sequence")。
#[test]
fn list_endpoints_decode_with_items_envelope() {
    let group_json = r#"{"items":[{"key":"g-uuid-1","name":"claude","ratio":1.2,
        "modelWhitelist":["claude-sonnet-4"],"remark":"Claude 专用","status":1}]}"#;
    let parsed: Items<GroupDto> =
        serde_json::from_str(group_json).expect("分组列表应能按 items 信封解析");
    assert_eq!(parsed.items.len(), 1);
    assert_eq!(
        parsed.items[0].key, "g-uuid-1",
        "信封剥壳后应能拿到定位用 key"
    );
    assert_eq!(parsed.items[0].name, "claude");

    // 渠道信封另带 total,剥壳只认 items,未知字段忽略。
    let channel_json = r#"{"items":[{"key":"c-uuid-9","name":"OpenAI 官方",
        "channelType":"openai","baseUrl":"https://api.openai.com/v1","keyCount":2,
        "models":[],"groups":["default"],"priority":0,"weight":0,"status":1,"remark":""}],
        "total":1}"#;
    let parsed: Items<ChannelDto> =
        serde_json::from_str(channel_json).expect("渠道列表应能按 items 信封解析(忽略 total)");
    assert_eq!(parsed.items[0].key, "c-uuid-9");
    assert_eq!(parsed.items[0].name, "OpenAI 官方");
}

/// 反例锁定:同一响应按裸 `Vec` 解码必须失败——这正是旧的
/// `list_groups_api`/`list_channels_api` 误标 `Vec<GroupDto>` 的报错现场。
#[test]
fn bare_vec_decode_of_envelope_response_fails() {
    let group_json = r#"{"items":[{"key":"g-uuid-1","name":"claude","ratio":1.2,
        "modelWhitelist":[],"remark":"","status":1}]}"#;
    let bare = serde_json::from_str::<Vec<GroupDto>>(group_json);
    assert!(
        bare.is_err(),
        "map 按裸 Vec 解必失败:{}",
        bare.err().unwrap()
    );
}
