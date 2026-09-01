//! 模型页的数据来源。面板只从这里取数,不认识数据是怎么来的。
//!
//! 现在是 mock 直连(同步返回 `mock` crate 的静态数据);接上真实后端时
//! 只改本文件 —— 换成 `crates/shared/client` 的请求,必要时把签名改成 async,
//! `models.rs` 的卡片渲染不用动。

// `ModelInfo.groups` 的行类型是 `mock::models::GroupPrice`;面板只读字段,不点名该类型,
// 所以这里不转出去(转了就是一条 unused 警告)。真要点名时从 `mock::models` 取。
pub use mock::models::ModelInfo;

pub fn fetch_models() -> &'static [ModelInfo] {
    mock::models::MODELS
}
