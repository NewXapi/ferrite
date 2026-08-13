//! ErrorKind 与 classify 约定：决定可重试 / 不可重试 / 该熔断。
//! 这张表错了会导致「key 坏了却一直重试」，见 docs/08-mvp.md §3.2。

/// 上游失败的分类。与 docs/08-mvp.md §1.9 的快照类别一一对应。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// 连不上、DNS 失败、TLS 握手失败
    Network,
    /// 连接 / 首字节 / 整体 三种超时之一
    Timeout,
    /// 上游拒绝：参数错、鉴权错
    Upstream4xx,
    /// 上游故障
    Upstream5xx,
    /// 429
    RateLimited,
    /// 格式转换出错
    ConvertFailed,
    /// 无可用渠道
    NoChannel,
    /// 流式中途断裂
    StreamBroken,
}

impl ErrorKind {
    /// 是否值得换个渠道重试。
    pub fn retryable(self) -> bool {
        matches!(
            self,
            Self::Network | Self::Timeout | Self::Upstream5xx | Self::RateLimited
        )
    }

    /// 是否应计入熔断。注意 4xx 里的鉴权失败也要熔断（key 坏了），
    /// 但参数错不该熔断 —— 那是客户端的问题。细分见 provider 各自的 classify。
    pub fn trips_breaker(self) -> bool {
        matches!(
            self,
            Self::Network | Self::Timeout | Self::Upstream5xx | Self::RateLimited
        )
    }
}
