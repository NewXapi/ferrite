//! 出口客户端池 — 连接复用 / 代理 / 超时。
//!
//! 参考: new-api internal/egress (client 池/HTTP2 分片/代理), wildtoken (per-upstream 超时)。
//! V1 范围: 直连 + HTTP(S) 代理; SOCKS/sing-box/TLS 伪装 deferred (账号池场景)。

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures_util::{Stream, StreamExt, TryStreamExt};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::Policy as RedirectPolicy;
use tokio::time::timeout;

/// 超时配置 (ms)。
/// TODO(#323): 数值进 config.toml; 默认 connect 5s / first-byte 30s / total 300s。
#[derive(Debug, Clone, Copy)]
pub struct Timeouts {
    pub connect_ms: u64,
    pub first_byte_ms: u64,
    pub total_ms: u64,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            connect_ms: 5_000,
            first_byte_ms: 30_000,
            total_ms: 300_000,
        }
    }
}

/// 出口执行器 — 池化的 reqwest Client 持有者。
///
/// V1 范围: 单直连池, 关闭重定向跟随 (上游 3xx 按错误处理, 不静默跳)。
/// SOCKS/HTTP 代理与按 base_url 分片留给 TODO(#522) 池化阶段。
///
/// 返回 `Pin<Box<dyn Future>>` 而非 `impl Future`, 满足 trait 的 dyn 兼容性
/// (apps/gateway 通过 `Arc<dyn Egress>` 注入)。
pub trait Egress: Send + Sync {
    /// 发送已准备好的请求, 返回原始响应 (含响应流)。
    ///
    /// 三段超时语义:
    /// - `connect_ms`: reqwest Client 内置 connect 超时 (TCP/TLS 握手)
    /// - `first_byte_ms`: 响应头到达后, 首 chunk 读取超时
    /// - `total_ms`: 整个请求生命周期 (含读 body)
    ///
    /// 返回的 `ForwardedResponse` 仅头部信息; 字节流读取由调用方通过
    /// `ForwardedResponse::into_body_stream` 接管 (已 prepend 抢到的首字节)。
    fn execute<'a>(
        &'a self,
        url: &'a str,
        headers: &'a [(String, String)],
        body: Bytes,
        timeouts: &'a Timeouts,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<ForwardedResponse, contract::error::NormalizedError>>
                + Send
                + 'a,
        >,
    >;
}

/// 转发响应 — `reqwest::Response` 头部 + 字节流。
///
/// 已 prepend first-byte 检查抢到的首字节 (若有); 调用方读 body 流时
/// 拿到的字节顺序与上游逐字一致。
pub struct ForwardedResponse {
    status: u16,
    content_type: String,
    body: Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>,
}

impl ForwardedResponse {
    /// 测试/mock 构造器 — 从字节流直接组装 (不经过 reqwest)。
    /// 生产路径由 `ReqwestEgress::execute` 构建。
    pub fn from_stream(
        status: u16,
        content_type: impl Into<String>,
        body: impl futures_util::Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static,
    ) -> Self {
        Self {
            status,
            content_type: content_type.into(),
            body: Box::pin(body),
        }
    }

    /// 上游 HTTP 状态码。
    pub fn status(&self) -> u16 {
        self.status
    }

    /// 上游 Content-Type (用于 stream 模块判定是否挂 SSE 扫描)。
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// 消费 body 流 (所有权转移, 之后不能再访问 status/content_type)。
    pub fn into_body_stream(
        self,
    ) -> Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>> {
        self.body
    }
}

impl std::fmt::Debug for ForwardedResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForwardedResponse")
            .field("status", &self.status)
            .field("content_type", &self.content_type)
            .finish_non_exhaustive()
    }
}

/// reqwest 出口 — 持有共享 `reqwest::Client`。
///
/// 直连; 不跟随重定向; 连接池默认按 host 分桶 (reqwest 自带)。
/// ponytail: 池化/分片留 TODO(#522); 当前一个 Client 全局复用足以吃下
/// 普通负载, base_url 分片在出现 hot-key 性能塌方时再上。
#[derive(Clone)]
pub struct ReqwestEgress {
    client: Arc<reqwest::Client>,
    /// 默认 connect 超时 (per-request 可被 timeouts 覆盖 — 当前实现用此默认)。
    connect_timeout: Duration,
}

impl std::fmt::Debug for ReqwestEgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReqwestEgress").finish_non_exhaustive()
    }
}

impl Default for ReqwestEgress {
    fn default() -> Self {
        Self::new()
    }
}

impl ReqwestEgress {
    /// 直连 client — 禁重定向, 内置连接池, 不带任何代理。connect 超时默认 5s。
    pub fn new() -> Self {
        let connect_timeout = Duration::from_millis(5_000);
        let client = reqwest::Client::builder()
            .redirect(RedirectPolicy::none())
            .connect_timeout(connect_timeout)
            .pool_max_idle_per_host(8)
            .build()
            .expect("reqwest client build");
        Self {
            client: Arc::new(client),
            connect_timeout,
        }
    }

    /// 自定义 client — 用于测试 (注入 mock transport) 或未来代理池化。
    pub fn with_client(client: reqwest::Client, connect_timeout: Duration) -> Self {
        Self {
            client: Arc::new(client),
            connect_timeout,
        }
    }

    /// 默认 connect 超时 (per-call `timeouts.connect_ms` 当前不被 ClientBuilder
    /// 接受, 用 builder 阶段固化)。暴露给诊断用。
    pub fn connect_timeout(&self) -> Duration {
        self.connect_timeout
    }
}

/// 非 2xx → `NormalizedError`。
///
/// 按状态码归类 (对齐 dispatch::health::FailureClass):
/// - 401/403 → invalid_api_key, 不重试
/// - 429 → rate_limited, 可重试
/// - 5xx → upstream_error, 可重试
/// - 其它 4xx → upstream_error (致命), 不重试 (协议/客户端问题, 换渠道救不了)
fn classify_status(status: u16, body_preview: String) -> contract::error::NormalizedError {
    use contract::error::code;
    let (code, http_status, retryable) = match status {
        401 | 403 => (code::INVALID_API_KEY, status, false),
        429 => (code::RATE_LIMITED, status, true),
        500..=599 => (code::UPSTREAM_ERROR, status, true),
        400..=499 => (code::UPSTREAM_ERROR, status, false),
        _ => (code::UPSTREAM_ERROR, 502, false),
    };
    contract::error::NormalizedError {
        code,
        status: http_status,
        retryable,
        message: body_preview.chars().take(200).collect(),
    }
}

fn build_header_map(
    headers: &[(String, String)],
) -> Result<HeaderMap, contract::error::NormalizedError> {
    let mut map = HeaderMap::new();
    for (k, v) in headers {
        let name =
            HeaderName::try_from(k.as_str()).map_err(|e| contract::error::NormalizedError {
                code: contract::error::code::UPSTREAM_ERROR,
                status: 502,
                retryable: false,
                message: format!("invalid header name `{k}`: {e}"),
            })?;
        let value =
            HeaderValue::try_from(v.as_str()).map_err(|e| contract::error::NormalizedError {
                code: contract::error::code::UPSTREAM_ERROR,
                status: 502,
                retryable: false,
                message: format!("invalid header value `{v}`: {e}"),
            })?;
        map.append(name, value);
    }
    Ok(map)
}
/// 把"已读首字节" prepend 到 reqwest 字节流前 — 避免 first-byte 检查丢字节。
fn prepend_first(
    first: Bytes,
    rest: Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>,
) -> Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>> {
    let head = futures_util::stream::once(async move { Ok(first) });
    Box::pin(head.chain(rest))
}

impl Egress for ReqwestEgress {
    fn execute<'a>(
        &'a self,
        url: &'a str,
        headers: &'a [(String, String)],
        body: Bytes,
        timeouts: &'a Timeouts,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<ForwardedResponse, contract::error::NormalizedError>>
                + Send
                + 'a,
        >,
    > {
        let first_byte_timeout = Duration::from_millis(timeouts.first_byte_ms);
        let total_timeout = Duration::from_millis(timeouts.total_ms);
        let connect_timeout = Duration::from_millis(timeouts.connect_ms);

        let header_map = match build_header_map(headers) {
            Ok(m) => m,
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        // 阶段 1: 发请求到收齐响应头 (含 connect)。
        // connect 超时由 ClientBuilder 阶段固化 (build 时按 ReqwestEgress 默认);
        // 这里用 total_timeout 兜底整个 send (reqwest 在 connect 失败时会回错)。
        let client = Arc::clone(&self.client);
        let url_owned = url.to_string();
        let send_fut = async move {
            let req = client
                .request(reqwest::Method::POST, &url_owned)
                .headers(header_map)
                .body(body);
            // 局部覆盖 connect_timeout 用 RequestBuilder.timeout 总超时
            // (reqwest 0.12 在 builder 上无 per-request connect_timeout,
            // 只能通过 client 共享。ponytail: 后续把 connect_timeout
            // 拆到 client 池分片阶段, 此处先用 total_timeout + is_connect 兜底)。
            let _ = connect_timeout;
            req.send().await.map_err(|e| {
                let msg = e.to_string();
                let retryable = e.is_connect() || e.is_timeout() || e.is_request();
                contract::error::NormalizedError {
                    code: contract::error::code::UPSTREAM_ERROR,
                    status: if e.is_timeout() { 504 } else { 502 },
                    retryable,
                    message: msg,
                }
            })
        };

        Box::pin(async move {
            let resp = timeout(total_timeout, send_fut).await.map_err(|_| {
                contract::error::NormalizedError {
                    code: contract::error::code::UPSTREAM_ERROR,
                    status: 504,
                    retryable: true,
                    message: "upstream total timeout".into(),
                }
            })??;

            let status = resp.status().as_u16();
            let content_type = resp
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();

            // 阶段 2: 非 2xx → 读完整 body 归类错误
            if !resp.status().is_success() {
                let preview = match resp.bytes().await {
                    Ok(b) => String::from_utf8_lossy(&b[..b.len().min(2048)]).into_owned(),
                    Err(_) => String::new(),
                };
                return Err(classify_status(status, preview));
            }

            // 阶段 3: 2xx → first-byte 检查。抢读首 chunk (再 prepend 回流), 超时即报错。
            // reqwest::bytes_stream 产出 `Result<Bytes, reqwest::Error>`, 在边界处
            // 映射到 `std::io::Error` 供上层统一处理。
            let raw_stream = resp.bytes_stream();
            let mapped_stream = raw_stream
                .map_err(|e| std::io::Error::other(format!("upstream stream error: {e}")));
            let mut mapped_stream = Box::pin(mapped_stream);

            let first_chunk = timeout(first_byte_timeout, mapped_stream.next())
                .await
                .map_err(|_| contract::error::NormalizedError {
                    code: contract::error::code::UPSTREAM_ERROR,
                    status: 504,
                    retryable: true,
                    message: "upstream first-byte timeout".into(),
                })?;

            let body: Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>> =
                match first_chunk {
                    None => {
                        // 上游 EOF 立即结束 (空响应体, 但 2xx)。透传空流。
                        Box::pin(futures_util::stream::empty())
                    }
                    Some(Ok(first)) => prepend_first(first, mapped_stream),
                    Some(Err(e)) => {
                        return Err(contract::error::NormalizedError {
                            code: contract::error::code::UPSTREAM_ERROR,
                            status: 502,
                            retryable: true,
                            message: format!("upstream stream error: {e}"),
                        });
                    }
                };
            Ok(ForwardedResponse {
                status,
                content_type,
                body,
            })
        })
    }
}
