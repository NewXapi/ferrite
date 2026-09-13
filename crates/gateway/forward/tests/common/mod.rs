//! forward 集成测试共享 mock 与请求构造器。
//!
//! 各测试文件以 `mod common; use common::*;` 引入。本模块是对
//! retry_wiring / failure_telemetry / settle 三文件内联副本的纯机械去重：
//! mock 逻辑与断言语义保持不变，`Plan`/`MockDispatch` 取各文件的超集形态
//! （记录字段对不读它的用例不可见）。common 随每个测试二进制单独编译，
//! 未在所有二进制中用到的项以 `#[allow(dead_code)]` 标注。

use bytes::Bytes;
use contract::error::NormalizedError;
use contract::records::{RouteUnitRecord, SyncMeta, UsageEventRecord};
use dispatch::health::FailureClass;
use dispatch::{Candidate, Dispatch, DispatchError};
use forward::egress::{Egress, ForwardedResponse, Timeouts};
use gateway_pipeline::ctx::{BodySource, ProtocolKind, RequestMeta, SelectedRoute, StreamedAccum};
use gateway_pipeline::{RequestCtx, TokenInfo};
use metering::SettleSink;
use std::sync::Mutex;

// ---------- 请求构造 ----------

/// 非流式最小请求体（三个测试文件共用的默认 body）。
pub const NON_STREAM_BODY: &[u8] = b"{\"model\":\"m\"}";

/// 流式请求体：含顶层 `"stream":true`，`body_wants_stream` 据此判定流式意图。
#[allow(dead_code)] // 仅 failure_telemetry 使用
pub const STREAM_BODY: &[u8] = b"{\"model\":\"m\",\"stream\":true}";

/// 构造测试候选：base_url 含 key 标记, ScriptedEgress 据此区分候选。
pub fn candidate(key: &str) -> Candidate {
    SelectedRoute {
        unit: RouteUnitRecord {
            meta: SyncMeta {
                key: key.to_string(),
                schema_version: 1,
                logical_version: 1,
                origin: "test".to_string(),
                updated_at: chrono::Utc::now(),
            },
            group: "g".to_string(),
            public_model: "m".to_string(),
            channel_key: format!("ch-{key}"),
            key_index: 0,
            upstream_model: "m".to_string(),
            priority: 10,
            weight: 10,
            status: 1,
        },
        secret: "sk-test".to_string(),
        base_url: format!("http://upstream-{key}.invalid"),
        upstream_model: "m".to_string(),
        provider_type: "openai".to_string(),
        settings: serde_json::Value::Null,
    }
}

/// 构造请求 ctx：body 决定流式意图（含 `"stream":true` 即流式），route 预置给
/// 单次模式用（with_retry 后 handle 忽略它）。token 固定 tok-1/g；
/// 渠道归因字段留空（#140：本组用例不依赖渠道归因）。
pub fn ctx_with_body(body: &'static [u8], route: Candidate) -> RequestCtx {
    RequestCtx {
        request: RequestMeta {
            method: "POST".to_string(),
            path: "/v1/chat/completions".to_string(),
            headers: http::HeaderMap::new(),
            body: BodySource::InMemory(Bytes::from_static(body)),
            client_ip: "127.0.0.1".parse().unwrap(),
            request_id: uuid::Uuid::now_v7(),
            inbound_protocol: ProtocolKind::OpenAI,
        },
        token: Some(TokenInfo {
            id: "tok-1".into(),
            group: "g".to_string(),
            enabled: true,
            allowed_models: None,
            auth_version: 1,
        }),
        requested_model: Some("m".to_string()),
        route: Some(route),
        drop_guards: Vec::new(),
        upstream: None,
        streamed: StreamedAccum::default(),
        selected_channel_key: None,
        selected_channel_name: None,
        error: None,
    }
}

/// 构造仅含 route 的请求 ctx（无 body 解析需求；retry-attribution 用例用）。
#[allow(dead_code)] // 仅 retry_wiring 使用
pub fn ctx_with_route(route: Candidate) -> RequestCtx {
    ctx_with_body(NON_STREAM_BODY, route)
}

// ---------- egress mock ----------

/// 脚本化 egress mock 的计划项：`Ok` 返回固定成功体，`Fail` 返回分类错误。
#[allow(dead_code)] // settle 二进制不使用本 mock
pub enum Plan {
    Ok { body: &'static [u8] },
    Fail { status: u16, retryable: bool },
}

/// 脚本化 egress mock：按 url 里的候选标记返回固定结果。
/// `Fail` 构造的 NormalizedError 与真实 egress::classify_status 的输出同构
/// （502 → retryable，400 → 非 retryable，401/403/404 → channel_scoped），
/// forward 只见 trait 返回值。
#[allow(dead_code)] // settle 二进制不使用本 mock
pub struct ScriptedEgress {
    plans: Vec<(String, Plan)>,
    calls: Mutex<Vec<String>>,
}

#[allow(dead_code)] // 同上：构造器与调用记录访问器并非所有二进制都用
impl ScriptedEgress {
    pub fn new(plans: Vec<(&str, Plan)>) -> Self {
        Self {
            plans: plans.into_iter().map(|(k, p)| (k.to_string(), p)).collect(),
            calls: Mutex::new(Vec::new()),
        }
    }

    /// 已请求的上游 url 序列（供断言尝试次数与命中候选）。
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl Egress for ScriptedEgress {
    fn execute<'a>(
        &'a self,
        url: &'a str,
        _headers: &'a [(String, String)],
        _body: Bytes,
        _timeouts: &'a Timeouts,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<ForwardedResponse, contract::error::NormalizedError>,
                > + Send
                + 'a,
        >,
    > {
        self.calls.lock().unwrap().push(url.to_string());
        let (_, plan) = self
            .plans
            .iter()
            .find(|(marker, _)| url.contains(marker))
            .unwrap_or_else(|| panic!("ScriptedEgress: unexpected url {url}"));
        match plan {
            Plan::Ok { body } => {
                let body = Bytes::from_static(body);
                let stream = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(body)]);
                Box::pin(async move {
                    Ok(ForwardedResponse::from_stream(
                        200,
                        "application/json",
                        stream,
                    ))
                })
            }
            Plan::Fail { status, retryable } => {
                let (status, retryable) = (*status, *retryable);
                let err = NormalizedError {
                    code: contract::error::code::UPSTREAM_ERROR,
                    status,
                    retryable,
                    channel_scoped: matches!(status, 401 | 403 | 404) && !retryable,
                    message: format!("upstream {status}"),
                };
                Box::pin(async move { Err(err) })
            }
        }
    }
}

/// 恒返 200 + 固定 body 的 mock egress（非流式）。
#[allow(dead_code)] // 仅 settle 使用
pub struct FixedEgress {
    pub body: &'static [u8],
}

impl Egress for FixedEgress {
    fn execute<'a>(
        &'a self,
        _url: &'a str,
        _headers: &'a [(String, String)],
        _body: Bytes,
        _timeouts: &'a Timeouts,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<ForwardedResponse, NormalizedError>>
                + Send
                + 'a,
        >,
    > {
        let stream = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(
            Bytes::from_static(self.body),
        )]);
        Box::pin(async move {
            Ok(ForwardedResponse::from_stream(
                200,
                "application/json",
                stream,
            ))
        })
    }
}

// ---------- dispatch mock ----------

/// 手写 mock Dispatch：按 exclude 顺序吐候选，并把每次 select 收到的
/// exclude 集与每次 report 原样记录，供断言接线行为（不读记录的用例
/// 忽略记录字段即可，select/report 判定逻辑与各文件原内联版本一致）。
#[allow(dead_code)] // settle 二进制不使用本 mock；记录字段仅 retry_wiring 读取
pub struct MockDispatch {
    candidates: Vec<Candidate>,
    selects: Mutex<Vec<Vec<String>>>,
    reports: Mutex<Vec<(String, Result<u16, FailureClass>)>>,
}

#[allow(dead_code)] // 同上：构造器与记录访问器并非所有二进制都用
impl MockDispatch {
    pub fn new(candidates: Vec<Candidate>) -> Self {
        Self {
            candidates,
            selects: Mutex::new(Vec::new()),
            reports: Mutex::new(Vec::new()),
        }
    }

    /// select 调用序列（每次的 exclude 集快照）。
    pub fn selects(&self) -> Vec<Vec<String>> {
        self.selects.lock().unwrap().clone()
    }

    /// report 调用序列（unit key + 健康结果）。
    pub fn reports(&self) -> Vec<(String, Result<u16, FailureClass>)> {
        self.reports.lock().unwrap().clone()
    }
}

impl Dispatch for MockDispatch {
    fn select(
        &self,
        group: &str,
        model: &str,
        exclude: &[String],
    ) -> Result<Candidate, DispatchError> {
        self.selects
            .lock()
            .unwrap()
            .push(exclude.iter().map(|s| s.to_string()).collect());
        self.candidates
            .iter()
            .find(|c| !exclude.contains(&c.unit.meta.key))
            .cloned()
            .ok_or_else(|| DispatchError::NoCandidate {
                group: group.to_string(),
                model: model.to_string(),
            })
    }

    fn report(&self, unit_key: &str, outcome: Result<u16, FailureClass>) {
        self.reports
            .lock()
            .unwrap()
            .push((unit_key.to_string(), outcome));
    }

    fn channel_name(&self, channel_key: &str) -> Option<String> {
        // candidate() 生成 `ch-{key}`；测试断言 `name-{key}`。
        let key = channel_key.strip_prefix("ch-")?;
        Some(format!("name-{key}"))
    }
}

// ---------- sink mock ----------

/// 内存 sink mock（apps 侧实现的同型物）：收集结算/观测事件供断言。
#[allow(dead_code)] // retry_wiring 二进制不使用本 mock
#[derive(Default)]
pub struct VecSink(Mutex<Vec<UsageEventRecord>>);

impl SettleSink for VecSink {
    fn submit(&self, event: UsageEventRecord) {
        self.0.lock().unwrap().push(event);
    }
}

#[allow(dead_code)] // 同上：events 访问器并非所有二进制都用
impl VecSink {
    pub fn events(&self) -> Vec<UsageEventRecord> {
        self.0.lock().unwrap().clone()
    }
}
