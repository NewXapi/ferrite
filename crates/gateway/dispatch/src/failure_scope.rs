//! P1-B 渠道相关性分类。401/403（凭据）、404（该渠道无此模型）换渠道
//! 可能成立 → Channel；400/413/422（请求本身坏）换谁都是失败 → Request。
//! 对标：api-hub 确定错误降层（:466-474）、sub2api NextAccountAction 三态。

/// 一次 4xx 失败是否值得换渠道再试。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureScope {
    /// 换渠道可能成立。
    Channel,
    /// 请求本身有问题，换渠道无效。
    Request,
}

/// 只吃"渠道相关 4xx"的归类。5xx/429 不走此函数（可重试分类在
/// `egress::classify_status` 已有）。
pub fn classify_channel_scope(status: u16) -> FailureScope {
    match status {
        401 | 403 | 404 => FailureScope::Channel,
        _ => FailureScope::Request,
    }
}
