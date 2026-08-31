//! 控制台页面的 mock 数据源:纯静态数据,零依赖。
//!
//! 面板不直接引用本 crate,而是各自经 page 侧的 `api.rs` 取用。
//! 接上真实后端后只改那些 `api.rs`,本 crate 随之退场。
//!
//! 范围:`account`(密钥·资料 / 用量日志 / 邀请奖励)、`users`(用户管理)、
//! `overview`(总览统计 / 用量分布)、`models`(模型卡片 / 分组价格)。
//! leaderboard 的数据用 `asset!()` 宏引立绘,留在面板内的 data 层(见 page-leaderboard)。

pub mod account;
pub mod models;
pub mod overview;
pub mod users;
