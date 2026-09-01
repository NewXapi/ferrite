//! 出口客户端池 — 连接复用 / 代理 / 超时。
//!
//! 参考: new-api internal/egress (client 池/HTTP2 分片/代理), wildtoken (per-upstream 超时)。
//! V1 范围: 直连 + HTTP(S) 代理; SOCKS/sing-box/TLS 伪装 deferred (账号池场景)。

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
        Self { connect_ms: 5_000, first_byte_ms: 30_000, total_ms: 300_000 }
    }
}

/// 出口执行器 — 池化的 reqwest Client 持有者。
pub trait Egress: Send + Sync {
    /// 发送已准备好的请求, 返回原始响应流 (未做协议转换)。
    fn execute(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: bytes::Bytes,
        timeouts: &Timeouts,
    ) -> impl Future<Output = Result<reqwest::Response, protocol::NormalizedError>> + Send;
}

// 池实现要点 (TODO(#522)):
// - 直连/代理两套 Client 池; 渠道绑定代理时取代理池;
// - 关闭重定向跟随 (上游 3xx 按错误处理, 不静默跳);
// - 池按 base_url 分片, 配置变更时重建 (invalidate 语义)。
