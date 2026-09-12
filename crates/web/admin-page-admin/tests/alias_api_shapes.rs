//! 别名页 models 域 API 的 serde 请求/响应形状断言。
//!
//! 后端 models 域(`crates/api/admin-catalog/src/models.rs`)请求体为
//! camelCase;本文件把前端发出的 JSON 形状锁定下来,防止字段漂移:
//! - update 走 contract `AliasUpsertRequest`,但后端 `UpdateModelRequest`
//!   只认 name/status 等注册列;其余字段序列化为 null 且被后端忽略。
//! - create 所需的 owner/api_key 在 `AliasUpsertRequest` 中不存在 —— 这是
//!   create 未接线的根因,用形状断言把该事实固定下来(等 contract 会话
//!   补 ModelCreateRequest 后再接线)。

use admin_page_admin::api::ModelAliasView;
use contract::api::billing::AliasUpsertRequest;

/// update 请求形状:只填 name 时,JSON 必须携带 camelCase 的 name,
/// 其余字段序列化为 null(后端 UpdateModelRequest 将它们读成 None)。
#[test]
fn update_request_serializes_name_with_null_rest() {
    let req = AliasUpsertRequest {
        name: "gpt-4o".into(),
        ..Default::default()
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["name"], "gpt-4o");
    assert_eq!(v["displayName"], serde_json::Value::Null);
    assert_eq!(v["inputPer1k"], serde_json::Value::Null);
    assert_eq!(v["outputPer1k"], serde_json::Value::Null);
    assert_eq!(v["multiplier"], serde_json::Value::Null);
    assert_eq!(v["status"], serde_json::Value::Null);
}

/// 镜像后端 UpdateModelRequest(admin-catalog models.rs,全 Option、
/// camelCase),验证 update 请求体能被后端反序列化且语义正确:
/// 仅 name 生效,owner/api_key 等保持 None(COALESCE 保留原值)。
/// 注意:本断言只锁 wire 形状(反序列化层);后端在 COALESCE 合并后还会
/// validate_model 整体校验 merged 值 — 存量 api_key 为空的行(无效数据)
/// 任何更新都会 400,与前端请求体形状无关,须先修复存量数据。
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
}

#[test]
fn update_request_is_accepted_by_update_model_request_shape() {
    let req = AliasUpsertRequest {
        name: "claude-sonnet-4".into(),
        ..Default::default()
    };
    let raw = serde_json::to_value(&req).unwrap();
    let mirror: UpdateModelRequestMirror = serde_json::from_value(raw).unwrap();
    assert_eq!(mirror.name.as_deref(), Some("claude-sonnet-4"));
    // name 之外的字段必须全部为 None:COALESCE 保留原值,任何回传都会覆盖
    // 服务端状态(尤其 api_key — 前端只有 maskedKey,回传即毁凭据)。
    assert!(mirror.owner.is_none());
    assert!(mirror.model_type.is_none());
    assert!(mirror.base_url.is_none());
    assert!(mirror.api_key.is_none(), "api_key 必须为 None 以保留原值");
    assert!(mirror.capabilities.is_none());
    assert!(mirror.speed.is_none());
    assert!(mirror.rating.is_none());
    assert!(mirror.max_tokens.is_none());
    assert!(mirror.is_vision.is_none());
    assert!(mirror.is_tool.is_none());
    assert!(mirror.status.is_none());
}

/// create 阻塞根因的形状证据:AliasUpsertRequest 序列化结果不含
/// owner/apiKey,而后端 CreateModelRequest 把两者声明为必填(非 Option、
/// 无 serde default)—— 该请求体 POST /api/models 必被拒,故 create 未接线。
#[test]
fn upsert_request_lacks_owner_and_api_key_required_by_create() {
    let req = AliasUpsertRequest {
        name: "gpt-4o".into(),
        ..Default::default()
    };
    let v = serde_json::to_value(&req).unwrap();
    assert!(v.get("owner").is_none(), "AliasUpsertRequest 缺 owner 字段");
    assert!(
        v.get("apiKey").is_none(),
        "AliasUpsertRequest 缺 apiKey 字段"
    );
}

/// 列表项视图与后端 ModelView 的读路径兼容性:用一份含全部 ModelView
/// 字段的样例 JSON(camelCase)断言 key/name 可解析;其余字段前端不消费,
/// serde 默认忽略未知字段。
#[test]
fn model_alias_view_parses_full_backend_model_view() {
    let raw = serde_json::json!({
        "key": "0b3f2c1a-0000-4000-8000-000000000001",
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
        "createdAt": "2026-09-12T00:00:00Z",
        "updatedAt": "2026-09-12T00:00:00Z"
    });
    let view: ModelAliasView = serde_json::from_value(raw).unwrap();
    assert_eq!(view.key, "0b3f2c1a-0000-4000-8000-000000000001");
    assert_eq!(view.name, "gpt-4o");
}

/// 列表项最小形状:GET /api/models 的 items 至少要有 key 与 name,
/// 两者缺一即解码失败(client 层返回 ApiError::Decode,页面走错误态)。
#[test]
fn model_alias_view_requires_key_and_name() {
    let ok: ModelAliasView =
        serde_json::from_value(serde_json::json!({"key": "k1", "name": "m1"})).unwrap();
    assert_eq!((ok.key.as_str(), ok.name.as_str()), ("k1", "m1"));

    let missing_name = serde_json::from_value::<ModelAliasView>(serde_json::json!({"key": "k1"}));
    assert!(missing_name.is_err(), "缺 name 必须解码失败");
    let missing_key = serde_json::from_value::<ModelAliasView>(serde_json::json!({"name": "m1"}));
    assert!(missing_key.is_err(), "缺 key 必须解码失败");
}
