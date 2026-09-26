//! 单色 stroke SVG 图标集（lucide 风格 24×24 viewBox）。
//!
//! 全部图标以 `stroke="currentColor"` + `fill="none"` 绘制，继承周围文字的
//! 颜色（含 dark: 变体），用于替代 emoji 承担界面视觉层；不引入图标字体或
//! 第三方图标 crate，path 数据逐字取自 lucide 对应图标，保持单色线性风格。
//!
//! 每个组件统一 props：
//! - `size`：像素尺寸（宽高相等，默认 16）；
//! - `class`：附加 Tailwind class，供尺寸/颜色微调用（如 `size-4 opacity-50`）。

mod arrow_right;
mod chart_bar;
mod chevron_left;
mod chevron_right;
mod copy;
mod dock;
mod log_out;
mod menu;
mod moon;
mod panel_right;
mod plus;
mod pop_out;
mod send;
mod settings;
mod stop;
mod sun;
mod trash;
mod user;
mod x;

pub use arrow_right::*;
pub use chart_bar::*;
pub use chevron_left::*;
pub use chevron_right::*;
pub use copy::*;
pub use dock::*;
pub use log_out::*;
pub use menu::*;
pub use moon::*;
pub use panel_right::*;
pub use plus::*;
pub use pop_out::*;
pub use send::*;
pub use settings::*;
pub use stop::*;
pub use sun::*;
pub use trash::*;
pub use user::*;
pub use x::*;
