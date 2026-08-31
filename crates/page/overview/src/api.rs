//! 总览页的数据来源。面板只从这里取数,不认识数据是怎么来的。
//!
//! 现在是 mock 直连(同步返回 `mock` crate 的静态数据);接上真实后端时
//! 只改本文件 —— 换成 `crates/shared/client` 的请求,必要时把签名改成 async,
//! `overview.rs` 无需改动。

/// 统计卡:(值, 标签)
pub fn fetch_stats() -> &'static [(&'static str, &'static str)] {
    mock::overview::STATS
}

/// 按工具分布:(名称, 用量, 占比 %)
pub fn fetch_tools() -> &'static [(&'static str, &'static str, f64)] {
    mock::overview::TOOLS
}

/// 按模型分布:(名称, 用量, 占比 %)
pub fn fetch_models() -> &'static [(&'static str, &'static str, f64)] {
    mock::overview::MODELS
}
