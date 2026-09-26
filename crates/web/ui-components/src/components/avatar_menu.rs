//! AvatarMenu — rust-ui (github.com/rust-ui/ui) 组件的复用层。
//!
//! 单个头像点击后向上弹出下拉菜单（popover），包含：
//! 1. Avatar 组件（图片/fallback/徽记）作为触发器（trigger），
//! 2. DropdownMenuContent 面板（使用 rust-ui 的 DropdownMenu），
//! 3. 向上弹出（position: Top）且可配置对齐（align: StartOuter/EndOuter）,
//! 4. 支持额外的菜单项（extra_items）和自定义回调（on_select, on_logout）,
//! 5. 通过 rust-ui 的 tw_merge + icons 依赖实现，与现有 ui-components 保持兼容。
//!
//! 用法示例：
//! AvatarMenu {
//!     user_name: "Dev Admin".to_string(),
//!     src: Some("avatar.jpg".to_string()),
//!     extra_items: vec![
//!         DropdownMenuItem { label: "账户资料", onclick: |_| (), href: "#account" },
//!         DropdownMenuItem { label: "退出登录", onclick: move |_| on_logout.call(()), variant: DropdownMenuActionVariant::Destructive },
//!     },
//!     on_select: |_| {},
//!     on_logout: on_logout,
//! }
//