//! 单次尝试转发管道 — forward 的核心编排。
//!
//! 管道 (一去一回):
//! ```text
//! ForwardTask
//!   → adapter::prepare (URL 拼接 / 鉴权头 / 渠道参数覆盖)
//!   → protocol::Registry::resolve (跨协议时转换请求体)
//!   → egress::execute (带超时发送)
//!   → [response]
//!       ├─ 非流式: 整体回传 + metering 结算
//!       └─ 流式:   stream::pipe (SseScanner → StreamScanner → 客户端)
//! ```
//!
//! 失败出口统一为 protocol::NormalizedError → dispatch::FailureClass。

use crate::ForwardTask;
use protocol::NormalizedError;

/// 管道执行 trait — apps/gateway 用 reqwest 实现。
pub trait Pipeline: Send + Sync {
    /// 执行一次转发尝试。不重试 (重试是 dispatch::retry 的事)。
    fn execute(
        &self,
        task: &ForwardTask,
        timeouts: &crate::egress::Timeouts,
    ) -> impl Future<Output = Result<crate::Forwarded, NormalizedError>> + Send;
}

/// 请求体预扫描 — metering 估算 prompt token 的输入挂点。
/// 在 adapt_request 之后、发送之前调用一次 (适配后的体才是上游真实体)。
/// TODO(#334): 与 metering::estimate_prompt_tokens 对接。
pub fn capture_prompt_body(_body: &bytes::Bytes) -> bytes::Bytes {
    bytes::Bytes::new()
}
