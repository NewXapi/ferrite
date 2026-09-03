//! # forward — 上游转发与协议适配 (热路径第 3 步)
//!
//! 职责: 把已准入、已选路的请求发往上游, 并把响应原样流回客户端。
//! 本 crate 只做 **IO 编排**; 字节格式转换全部委托 protocol crate。
//!
//! ## 模块地图
//!
//! | 模块 | 职责 |
//! |------|------|
//! | [`pipeline`] | 单次尝试的完整转发管道 (构建请求→发送→流回传) |
//! | [`adapter`]  | 上游请求准备: URL/头/鉴权 (new-api adaptor 五方法里的 3 个) |
//! | [`egress`]   | 出口客户端池: 代理/超时/HTTP2 分片 |
//! | [`stream`]   | SSE 双向管道: 上游帧 → protocol::SseScanner → metering::StreamScanner → 客户端 |
//!
//! ## 设计要点 (调查结论)
//!
//! 1. **透传优先**: 同协议 = 字节透传零转换; 跨协议才走 protocol::Registry;
//! 2. **流式优先**: Bytes 块管道, 不缓冲整个响应;
//! 3. **重试边界**: 本层只报 FailureClass 给 dispatch::retry, 不自行换候选;
//! 4. **可重放**: 请求体缓存 (Bytes 池), 重试时免读原始 body。

pub mod adapter;
pub mod egress;
pub mod pipeline;
pub mod stream;

use bytes::Bytes;

/// 一次转发任务的全部输入 (apps/gateway 组装)。
#[derive(Debug, Clone)]
pub struct ForwardTask {
    /// 已选候选 (dispatch::Candidate)。
    pub candidate: dispatch::Candidate,
    /// 客户端原始请求路径 (如 /v1/chat/completions)。
    pub path: String,
    /// 需要透传的客户端头 (已过滤 hop-by-hop/鉴权头)。
    pub headers: Vec<(String, String)>,
    /// 请求体 (可重放)。
    pub body: Bytes,
    /// 是否 SSE (客户端 Accept 判定)。
    pub stream: bool,
}

/// 转发结果: 上游状态码 + 响应流。
pub struct Forwarded {
    pub status: u16,
    /// 响应字节流 (SSE 或普通 body 统一形状)。
    pub body: Box<dyn futures_util::Stream<Item = Result<Bytes, std::io::Error>> + Send + Unpin>,
    /// 响应内容类型 (决定 stream 模块是否挂 SSE 扫描)。
    pub content_type: String,
}
