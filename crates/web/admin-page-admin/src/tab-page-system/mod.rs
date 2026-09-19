//! 系统页 tab。拆分约定:
//! - `page`:状态 + 拉取 effect + 三个派生 Vec + 组件组合
//! - `shared`:`SystemInfoView` 等本地 DTO + 区段文案常量
//!   + 运行时长/字节/负载等纯函数格式化(内部共享,不扩进 crate 公开面)
//! - `options`:站点选项面板 + `format_option_value`
//!   / `option_editable`(原先即 pub)
//! - `proxy_nodes`:出口代理节点导入面板 + 导入 DTO
//! - `proxy_runtime`:代理节点运行态面板 + report DTO
//!   (三个 DTO 原先即 pub)+ 冷却/延迟格式化

#[path = "options.rs"]
pub mod options;
#[path = "overview.rs"]
pub mod overview;
#[path = "page.rs"]
pub mod page;
#[path = "proxy-nodes.rs"]
pub mod proxy_nodes;
#[path = "proxy-runtime.rs"]
pub mod proxy_runtime;
#[path = "shared.rs"]
pub mod shared;

pub use options::*;
pub use page::*;
pub use proxy_nodes::*;
pub use proxy_runtime::*;
