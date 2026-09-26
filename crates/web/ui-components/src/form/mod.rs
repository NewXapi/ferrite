//! 表单基元：字段包装、统一输入字段、验证码字段、滑杆与密码字段。

pub const INPUT_CLASS: &str = "w-full rounded-lg border border-zinc-800 bg-zinc-900/80 px-3.5 py-2.5 text-sm text-zinc-100 placeholder-zinc-600 outline-none transition-all duration-200 hover:border-zinc-700 focus:border-zinc-500 focus:ring-2 focus:ring-zinc-500/20 focus:bg-zinc-900";

mod code_field;
mod field;
mod form_field;
mod password_field;
mod slider_field;

pub use code_field::*;
pub use field::*;
pub use form_field::*;
pub use password_field::*;
pub use slider_field::*;
