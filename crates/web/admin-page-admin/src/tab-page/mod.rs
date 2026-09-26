//! 页面编排层:每 tab 一个页面文件(spec 理念 1)。
//! 组件在 `crate::components`,跨页复用在 `ui-components`,wire 在 `crate::api`。

pub mod aliases;
pub mod channels;
pub mod currency;
pub mod entities;
pub mod gateway;
pub mod groups;
pub mod network;
pub mod redemptions;
pub mod subscriptions;
pub mod system;
