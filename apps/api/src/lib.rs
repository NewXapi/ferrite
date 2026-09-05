//! Ferrite — API gateway 核心逻辑
//! 分离为 lib crate 让集成测试可以访问内部模块

pub mod adapter;
pub mod billing;
pub mod config;
pub mod dispatch;
pub mod gateway;
pub mod identity;
pub mod ratelimit;
pub mod tavern;
