use dioxus::prelude::*;

/// 圆形头像: 有图显示图，无图显示首字符
///
/// shadcn 对齐迁移后委托 [`crate::avatar::Avatar`]（Root span +
/// avatar-image / avatar-fallback 解剖）。兼容策略：旧默认 `"h-9 w-9"`（36px）
/// 与 shadcn 默认 `"size-8"`（32px）不同档，映射到等价档 `"size-9"`（36px）保
/// 页面视觉零变化；调用方显式传入的 `size` 原样透传。
#[component]
pub fn Avatar(
    name: String,
    #[props(default)] src: Option<String>,
    #[props(default = "h-9 w-9".to_string())] size: String,
) -> Element {
    // 旧默认档（36px）映射到 shadcn 尺寸体系的等价档 size-9；
    // 其余显式值（含旧式 h-N w-N）原样透传，尺寸语义由新组件的 class 拼接保持。
    let size = if size == "h-9 w-9" {
        Some("size-9".to_string())
    } else {
        Some(size)
    };
    rsx! {
        crate::avatar::Avatar { name: name, src: src, size: size }
    }
}
