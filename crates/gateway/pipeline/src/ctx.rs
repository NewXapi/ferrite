//! `ctx` —— 跨 stage 共享的请求上下文
//!
//! 设计要点：
//! - `RequestCtx` 只能在当前请求内存在，**不允许跨请求共享**。
//! - 跨请求共享的状态（token 表 / 路由索引）放在 `ArcSwap` 中，由各 stage 显式 `.load()`。
//! - `StageOutcome` 三态枚举：Continue / ShortCircuit / Stream（定义在 `stage`）。

use std::net::IpAddr;

use crate::stage::StageError;
use bytes::Bytes;
use http::HeaderMap;
use uuid::Uuid;

/// 客户端请求使用的协议
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtocolKind {
    OpenAI,
    Anthropic,
    Gemini,
    OpenAIResp,
}

/// 请求体来源（一次性读取 / 内存缓冲 / 磁盘文件）
///
/// 每个上游 attempt 拿一个新的 `Reader`，避免读到 EOF。
/// 内存时是 `Bytes::clone()`，磁盘时是 `File::open()` + offset。
#[derive(Debug, Clone)]
pub enum BodySource {
    InMemory(Bytes),
    OnDisk { path: std::path::PathBuf, len: u64 },
}

impl BodySource {
    /// 拿一个新的读取游标（用于重试 / 多次 attempt）
    pub fn reader(&self) -> BodyReader<'_> {
        match self {
            BodySource::InMemory(b) => BodyReader::Memory(b),
            BodySource::OnDisk { path, len } => BodyReader::Disk { path, len: *len },
        }
    }
}

pub enum BodyReader<'a> {
    Memory(&'a Bytes),
    Disk {
        path: &'a std::path::PathBuf,
        len: u64,
    },
}

/// 不可变请求入参
#[derive(Debug, Clone)]
pub struct RequestMeta {
    pub method: String,
    pub path: String,
    pub headers: HeaderMap,
    pub body: BodySource,
    pub client_ip: IpAddr,
    pub request_id: Uuid,
    pub inbound_protocol: ProtocolKind,
}

/// 路由选路产物（Dispatch 写入）—— 一个可直接转发的完整目标。
///
/// 解析完成度是本类型的核心不变量：forward 拿到它之后**不需要再查任何快照**。
/// `provider_type` 与 `settings` 来自 `ChannelRecord`，由 dispatch 的
/// `resolve_candidate` 在选路时一并解析，避免 forward 反向依赖 catalog 快照。
///
/// 为什么定义在 `pipeline` 而不是 `dispatch`：`pipeline` 是全部 stage crate 的
/// 公共基座，反向不依赖任何具体 stage（见 crate 文档）。路由产物要经 `RequestCtx`
/// 跨 stage 传递，只能落在基座里；`dispatch::Candidate` 是本类型的别名。
///
/// `channel_key`（`unit.channel_key`）= `ChannelRecord.meta.key`，是渠道出口代理
/// 的租约键。ProxyManager::acquire(channel_key) 据此选节点。
#[derive(Debug, Clone)]
pub struct SelectedRoute {
    /// 命中的路由单元（原始记录，供计量/日志引用）。
    pub unit: contract::records::RouteUnitRecord,
    /// 解析后的上游凭据（`channel.keys[key_index].secret`）。
    pub secret: String,
    /// 上游 base_url（路径拼接规则见 `forward::adapter::build_url`）。
    pub base_url: String,
    /// 上游真名（`unit.upstream_model`），发往上游的 model 字段用它。
    pub upstream_model: String,
    /// 渠道协议族（`ChannelRecord.provider_type`）："openai" / "claude" /
    /// "gemini" / "passthrough"，决定鉴权头与路径模板。
    pub provider_type: String,
    /// 渠道级覆盖（`ChannelRecord.settings`）；`headers` 子集由 forward 消费，
    /// 其余留给计量与审计。
    pub settings: serde_json::Value,
}

/// 上游响应（非流式，Forward 写入）
#[derive(Debug)]
pub struct UpstreamResponse {
    pub status: u16,
    pub body: Bytes,
}

/// 流式累计（StreamingIntercept 写入）
#[derive(Debug, Default, Clone)]
pub struct StreamedAccum {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub first_token_at: Option<std::time::Instant>,
}

/// 流式响应（Forward → 客户端接管）
///
/// 包装 axum Body 与上游的 `content-type`；`gateway-forward` 的 SsePipe 通过
/// [`PipeStream::with_content_type`] / [`PipeStream::new`] 构造。
#[derive(Debug)]
pub struct PipeStream {
    body: axum::body::Body,
    content_type: String,
}

impl PipeStream {
    /// 由上游流构造，显式给出回给客户端的 `content-type`。
    ///
    /// 必须带上：SSE 客户端（OpenAI / Anthropic SDK）靠 `text/event-stream`
    /// 判定要按事件流读，缺了它会把响应当普通 body 一次性收完。
    pub fn with_content_type(body: axum::body::Body, content_type: impl Into<String>) -> Self {
        Self {
            body,
            content_type: content_type.into(),
        }
    }

    /// 由上游 SSE 流构造（forward 调用），content-type 取 `text/event-stream`。
    pub fn new(body: axum::body::Body) -> Self {
        Self::with_content_type(body, "text/event-stream")
    }

    /// 转换为 axum Response，带回上游的 `content-type`。
    pub fn into_response(self) -> http::Response<axum::body::Body> {
        http::Response::builder()
            .header(http::header::CONTENT_TYPE, &self.content_type)
            .body(self.body)
            // header 值来自上游 content-type 字符串；非法值退回无头响应而不是 panic。
            .unwrap_or_else(|_| http::Response::new(axum::body::Body::empty()))
    }
}

/// 跨 stage 共享的可变上下文
///
/// 每个字段在特定 stage 之后才被填充；未填充的字段是 None。
pub struct RequestCtx {
    /// 不可变入参
    pub request: RequestMeta,

    /// Admission 写入：已鉴权的 token 元数据
    pub token: Option<super::TokenInfo>,

    /// Dispatch 写入：选中的路由
    pub route: Option<SelectedRoute>,

    /// Gate::model 写入：请求体解析出的模型名 (dispatch 用它 lookup)。
    pub requested_model: Option<String>,

    /// Forward 写入：非流式响应体
    pub upstream: Option<UpstreamResponse>,

    /// StreamingIntercept 写入：流式累计
    pub streamed: StreamedAccum,

    /// 任意 stage 可写入：跨 stage 错误（不直接返回，用 StageOutcome 处理）
    pub error: Option<StageError>,
}

impl RequestCtx {
    pub fn new(request: RequestMeta) -> Self {
        Self {
            request,
            token: None,
            route: None,
            requested_model: None,
            upstream: None,
            streamed: StreamedAccum::default(),
            error: None,
        }
    }

    /// 从 axum Request 构造（apps/gateway 入口）
    ///
    /// 提取 method / path / headers / body / client_ip / request_id，
    /// 按路径推断协议类型。
    pub async fn from_axum(req: http::Request<axum::body::Body>) -> anyhow::Result<Self> {
        let (parts, body) = req.into_parts();
        let body_bytes = axum::body::to_bytes(body, usize::MAX).await?;

        let client_ip = parts
            .headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .and_then(|s| s.trim().parse().ok())
            .or_else(|| {
                parts
                    .extensions
                    .get::<std::net::SocketAddr>()
                    .map(|a| a.ip())
            })
            .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));

        let path = parts.uri.path().to_string();
        let inbound_protocol = detect_protocol(&path);

        let request = RequestMeta {
            method: parts.method.to_string(),
            path,
            headers: parts.headers,
            body: BodySource::InMemory(body_bytes),
            client_ip,
            request_id: Uuid::now_v7(),
            inbound_protocol,
        };
        Ok(Self::new(request))
    }
}

/// 按路径前缀推断入站协议
fn detect_protocol(path: &str) -> ProtocolKind {
    if path.starts_with("/v1/messages") {
        ProtocolKind::Anthropic
    } else if path.starts_with("/v1/responses") {
        ProtocolKind::OpenAIResp
    } else if path.contains("/v1beta") || (path.contains("/v1/") && path.contains("gemini")) {
        ProtocolKind::Gemini
    } else {
        ProtocolKind::OpenAI
    }
}
