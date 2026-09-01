//! # forward — 上游转发与协议适配 (热路径第 3 步)
//!
//! 职责: 把已准入、已选路的请求发往上游, 并把响应原样流回客户端。
//! 设计要点:
//!
//! 1. **透传优先**: 客户端发的是 OpenAI 格式 → 优先找"本来就是 OpenAI 协议"
//!    的渠道直接透传字节, 零转换; 只有协议不匹配时才走 [`Adapter`] 转换。
//!    (new-api 的经验: 大部分流量是 openai→openai, 转换纯属浪费)
//! 2. **流式优先**: SSE 一边读一边写 (bytes::Bytes 块管道), 不缓冲整个响应。
//!    usage 计数由 metering 的流扫描器在管道中并行完成, 本模块不管计费。
//! 3. **超时与重试边界**: 单次连接超时 / 首包超时在此层执行;
//!    "换候选重试"不在本层 (那是 dispatch 状态机的事, 本层只报 FailureClass)。

use bytes::Bytes;
use futures_util::Stream;



/// 一次转发任务的全部输入 (apps/gateway 组装)。
pub struct ForwardTask {
    // TODO(#321): 字段定型 — candidate, method, path, headers(过滤 hop-by-hop), body。
}

/// 转发结果: 上游状态码 + 响应流 (SSE 或普通 body)。
pub struct Forwarded {
    pub status: u16,
    // TODO(#322): 流类型 — BoxStream<Bytes> 加上 content-type 判断, 决定是否按 SSE 扫描。
}

/// 协议适配器: 仅在客户端格式 ≠ 上游格式时使用。
///
/// 实现者: OpenAiAdapter / ClaudeAdapter / GeminiAdapter。
/// 转换必须是**单遍流式**的: 不允许把整个请求/响应物化成 String 再解析
/// (new-api 的 map[string]interface{} 教训 — GC 压力大且丢字段)。
pub trait Adapter: Send + Sync {
    /// 请求体转换 (一次性, 请求通常较小)。
    fn adapt_request(&self, body: Bytes) -> Result<Bytes, AdapterError>;
    /// 响应流转换 (流式, 逐块)。
    fn adapt_response(
        &self,
        upstream: Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send + Unpin>,
    ) -> Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send + Unpin>;
}

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("malformed request body: {0}")]
    Malformed(String),
    /// 无法识别的上游协议变体 — 记录原始 body 片段供排查。
    #[error("unsupported upstream shape")]
    Unsupported,
}

/// 单次上游调用的超时配置 (毫秒)。
/// TODO(#323): 数值进 config.toml; 先给保守默认 (connect 5s, first-byte 30s, total 300s)。
pub struct Timeouts {
    pub connect_ms: u64,
    pub first_byte_ms: u64,
    pub total_ms: u64,
}
