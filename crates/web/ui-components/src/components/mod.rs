//! rust-ui registry 拷贝线过渡层（rui_ 前缀）。
//!
//! 上游 rust-ui-keys-replace 已拆除其余 rui_* 组件；仅剩 `rui_avatar`
//! 在用（layout/avatar_menu 引用）。替换完成后随目录一起删除。

pub mod rui_avatar;
