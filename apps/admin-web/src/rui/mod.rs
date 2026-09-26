//! rui — rust-ui (github.com/rust-ui/ui) registry 组件的拷贝层。
//!
//! shadcn copy-paste 模式：源码属于本项目，可自由修改；只追加依赖面
//! `tw_merge` + `icons`(Lucide)。与 crate 内自建 `ui-components` 并存，
//! 命名空间隔离避免混淆。组件清单见 todo/web-ui-ref/rust-ui/app_crates/registry/src/ui/。
pub mod button;
pub mod card;
pub mod dialog;
pub mod input;
pub mod tooltip;
