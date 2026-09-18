//! 别名页定价写回的 wire 层单测：定价三字段（input_per_1k / output_per_1k /
//! multiplier）在「 hydrate 读取 → 请求体序列化 → 响应回填」整条链路上的
//! JSON 形状与 Option 语义。
//!
//! 纯 DTO/序列化断言，不依赖网络与 Dioxus runtime。背景：后端 0018 迁移
//! 给 api_models 加了三列（commit 7e91063，admin-catalog `ModelView` 恒返回
//! 非 Option f64、`UpdateModelRequest` 全 Option + COALESCE），本文件把前端
//! 三处对齐该契约的地方钉死，防漂移：
//! - `AliasUpsertRequest` 带三定价字段的序列化形状（camelCase + Option 语义）；
//! - `ModelAliasView` 解码后端 `ModelView` 的三字段（不再硬填 0/1.0）；
//! - 别名 hydrate 的就地刷新断言：保存写回的请求体确实含三字段。

use admin_page_admin::api::ModelAliasView;
use contract::api::billing::AliasUpsertRequest;

/// 后端 `ModelView` 真实输出形状（camelCase，三定价字段恒为数字）。
fn sample_model_view_json() -> serde_json::Value {
    serde_json::json!({
        "key": "0b3f2c1a-0000-4000-8000-0000000000a1",
        "name": "gpt-4o",
        "owner": "admin",
        "modelType": "chat",
        "baseUrl": "https://api.example.com/v1",
        "maskedKey": "sk-a****bcde",
        "capabilities": {},
        "speed": 0,
        "rating": {},
        "usageCount": 0,
        "maxTokens": 0,
        "isVision": false,
        "isTool": false,
        "status": 1,
        "inputPer1k": 0.0175,
        "outputPer1k": 0.07,
        "multiplier": 1.5,
        "createdAt": "2026-09-12T00:00:00Z",
        "updatedAt": "2026-09-12T00:00:00Z"
    })
}

/// 三定价字段真实值必须被解码——这是 hydrate 不再硬填 0/1.0 的形状证据：
/// 若前端 view 缺这三字段，别名页卡片与统计卡会把已定价模型误算成
/// 「标准 1.0× / 免费别名」，编辑回填也拿不到现值。
#[test]
fn model_alias_view_decodes_real_pricing_fields() {
    let view: ModelAliasView = serde_json::from_value(sample_model_view_json()).unwrap();
    assert_eq!(view.key, "0b3f2c1a-0000-4000-8000-0000000000a1");
    assert_eq!(view.name, "gpt-4o");
    assert!((view.input_per_1k - 0.0175).abs() < 1e-12);
    assert!((view.output_per_1k - 0.07).abs() < 1e-12);
    assert!(
        (view.multiplier - 1.5).abs() < 1e-12,
        "倍率须取后端真值 1.5"
    );
}

/// 未定价（0018 列 DEFAULT）与无加价（DEFAULT 1.0）口径：三字段缺席时
/// 前端按 0/0/1.0 容错解码，与后端列默认值对齐，不整体解码失败。
#[test]
fn model_alias_view_defaults_missing_pricing_fields() {
    let view: ModelAliasView = serde_json::from_value(serde_json::json!({
        "key": "k1",
        "name": "unpriced-model"
    }))
    .expect("key+name 应足够解码");
    assert_eq!(view.key, "k1");
    assert_eq!(view.name, "unpriced-model");
    assert_eq!(view.input_per_1k, 0.0);
    assert_eq!(view.output_per_1k, 0.0);
    assert_eq!(view.multiplier, 1.0, "倍率缺省须回落 1.0（无加价），不是 0");
}

/// 编辑保存的请求体形状：name + 三定价字段，camelCase，其余字段序列化为 null
/// （被后端 `UpdateModelRequest` 的未知字段忽略 / 读成 None）。
#[test]
fn update_request_serializes_pricing_fields_camel_case() {
    let req = AliasUpsertRequest {
        name: "gpt-4o".into(),
        input_per_1k: Some(0.0175),
        output_per_1k: Some(0.07),
        multiplier: Some(1.5),
        ..Default::default()
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["name"], "gpt-4o");
    assert_eq!(v["inputPer1k"], 0.0175);
    assert_eq!(v["outputPer1k"], 0.07);
    assert_eq!(v["multiplier"], 1.5);
    // snake_case 键不得出现：后端按 camelCase 反序列化，蛇形键被当未知字段
    // 丢弃，对应列静默保持旧值（定价就白改了）。
    // 注：`multiplier` 是单词，camelCase 与 snake_case 同形，故只钉另两字段。
    assert!(v.get("input_per_1k").is_none());
    assert!(v.get("output_per_1k").is_none());
    // 不回传的身份字段保持 None：后端 COALESCE 保留 owner/api_key 现值
    // （前端只有 maskedKey，回传即毁凭据）。
    assert_eq!(v["displayName"], serde_json::Value::Null);
    assert_eq!(v["status"], serde_json::Value::Null);
}

/// Option 语义钉死：清空输入框 → 字段缺席（null）→ 后端 COALESCE 保持现值，
/// 而不是写零。这是「清空价格保存不会把模型改成免费」的形状保证。
#[test]
fn update_request_absent_pricing_fields_serialize_as_null() {
    let req = AliasUpsertRequest {
        name: "gpt-4o".into(),
        input_per_1k: None,
        output_per_1k: None,
        multiplier: None,
        ..Default::default()
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["inputPer1k"], serde_json::Value::Null);
    assert_eq!(v["outputPer1k"], serde_json::Value::Null);
    assert_eq!(v["multiplier"], serde_json::Value::Null);
}

/// 镜像后端 `UpdateModelRequest`（admin-catalog models.rs，全 Option、
/// camelCase、含 0018 三列），验证带定价的请求体能被后端反序列化且语义正确：
/// 三字段落到对应槽位，身份字段保持 None（COALESCE 保留原值）。
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateModelRequestMirror {
    name: Option<String>,
    owner: Option<String>,
    model_type: Option<String>,
    base_url: Option<String>,
    api_key: Option<String>,
    capabilities: Option<serde_json::Value>,
    speed: Option<i32>,
    rating: Option<serde_json::Value>,
    max_tokens: Option<i32>,
    is_vision: Option<bool>,
    is_tool: Option<bool>,
    status: Option<i16>,
    input_per_1k: Option<f64>,
    output_per_1k: Option<f64>,
    multiplier: Option<f64>,
}

#[test]
fn priced_update_request_is_accepted_by_backend_shape() {
    let req = AliasUpsertRequest {
        name: "claude-sonnet-4".into(),
        input_per_1k: Some(0.021),
        output_per_1k: Some(0.105),
        multiplier: Some(1.2),
        ..Default::default()
    };
    let raw = serde_json::to_value(&req).unwrap();
    let mirror: UpdateModelRequestMirror = serde_json::from_value(raw).unwrap();
    assert_eq!(mirror.name.as_deref(), Some("claude-sonnet-4"));
    assert_eq!(mirror.input_per_1k, Some(0.021));
    assert_eq!(mirror.output_per_1k, Some(0.105));
    assert_eq!(mirror.multiplier, Some(1.2));
    // 身份字段必须全 None：别名页只有 maskedKey，回传会覆盖服务端凭据。
    assert!(mirror.owner.is_none());
    assert!(mirror.api_key.is_none(), "api_key 必须为 None 以保留原值");
    assert!(mirror.model_type.is_none());
    assert!(mirror.base_url.is_none());
    assert!(mirror.capabilities.is_none());
    assert!(mirror.speed.is_none());
    assert!(mirror.rating.is_none());
    assert!(mirror.max_tokens.is_none());
    assert!(mirror.is_vision.is_none());
    assert!(mirror.is_tool.is_none());
    assert!(mirror.status.is_none());
}

/// 保存成功后的就地刷新链路：PUT 响应是后端 `ModelView`，前端按 view 的
/// key 定位列表行并以服务端值为准回填——本断言锁「响应解码 → 行字段」的
/// 对齐（页面侧的 signal 写入由 `AliasFormModal` 的提交回调完成，此处钉
/// 死其数据源形状）。
#[test]
fn put_response_view_refreshes_row_by_key() {
    // 旧硬填默认值的行（hydrate 前的本地占位）。
    let stale: ModelAliasView = serde_json::from_value(serde_json::json!({
        "key": "0b3f2c1a-0000-4000-8000-0000000000a1",
        "name": "gpt-4o"
    }))
    .unwrap();
    assert_eq!(stale.multiplier, 1.0);
    // 保存返回的落库视图。
    let saved: ModelAliasView = serde_json::from_value(sample_model_view_json()).unwrap();
    // 刷新语义：按 key 匹配（不依赖下标），三字段以服务端值为准。
    assert_eq!(stale.key, saved.key, "刷新必须按 key 定位，而非下标");
    assert_ne!(
        saved.multiplier, stale.multiplier,
        "服务端值必须覆盖本地占位值"
    );
    assert!((saved.input_per_1k - 0.0175).abs() < 1e-12);
    assert!((saved.output_per_1k - 0.07).abs() < 1e-12);
}
