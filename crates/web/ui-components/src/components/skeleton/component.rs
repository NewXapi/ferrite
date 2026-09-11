//! Skeleton — shadcn new-york-v4 风格骨架屏基元。
//!
//! class 与 data-slot 属性逐字取自 shadcn ui/skeleton.tsx
//! （`animate-pulse rounded-md bg-accent`）；调用方传入的 `class` 由 [`with_class`]
//! 追加到基串之后，其余属性原样透传。

use dioxus::core::AttributeValue;
use dioxus::prelude::*;

/// shadcn ui/skeleton.tsx 基础 class（逐字）。
pub const SKELETON_CLASS: &str = "animate-pulse rounded-md bg-accent";

/// 把调用方传入的 `class` 追加到组件 shadcn 基串之后，其余属性原样保留。
///
/// 与 button/badge 的 `with_class` 同签名同语义（各文件私有副本，见 button 的注释）。
fn with_class(attributes: Vec<Attribute>, extra: &str) -> Vec<Attribute> {
    let mut caller_class = String::new();
    let mut rest = Vec::with_capacity(attributes.len() + 1);
    for attribute in attributes {
        match (&attribute.value, attribute.name) {
            (AttributeValue::Text(value), "class") if !value.trim().is_empty() => {
                if !caller_class.is_empty() {
                    caller_class.push(' ');
                }
                caller_class.push_str(value.trim());
            }
            _ => rest.push(attribute),
        }
    }
    let class = if caller_class.is_empty() {
        extra.to_string()
    } else {
        format!("{extra} {caller_class}")
    };
    rest.push(Attribute::new("class", class, None, false));
    rest
}

/// shadcn new-york-v4 风格骨架屏。
///
/// ```ignore
/// Skeleton { class: "h-4 w-32" }
/// Skeleton {
///     div { class: "space-y-2", "自定义子节点" }
/// }
/// ```
#[component]
pub fn Skeleton(
    /// 透传到骨架 div 的属性；`class` 追加在 shadcn 基串之后（尺寸由调用方给）。
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    /// 可选子节点：shadcn 骨架通常是空 div，尺寸靠 class；也允许包内容。
    children: Option<Element>,
) -> Element {
    let class = with_class(attributes, SKELETON_CLASS);

    rsx! {
        div {
            "data-slot": "skeleton",
            ..class,
            {children}
        }
    }
}
