//! 本 crate 独有的 `#[component]` 组件(spec 理念 1:components/ 只放组件)。
//!
//! 文件名按 `<tab>_<语义>.rs` 前缀分组;纯函数与文案常量在根级模块,不进本目录。

pub mod gateway_health_row;

pub use gateway_health_row::GatewayHealthRow;
