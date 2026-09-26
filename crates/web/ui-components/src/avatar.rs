//! Avatar — shadcn new-york-v4 风格头像基元。
//!
//! shadcn 解剖（registry/new-york-v4/ui/avatar.tsx）：Root span
//! （`data-slot="avatar"`）作外壳，有 `src` 渲染 img
//! （`data-slot="avatar-image"`），无 `src` 渲染 Fallback
//! （`data-slot="avatar-fallback"`，显示 `name` 首字符）。
//!
//! class 契约（逐字来源 shadcn new-york-v4 ui/avatar.tsx）：
//! - Root：`relative flex {size} shrink-0 overflow-hidden rounded-full select-none`，
//!   尺寸段由 [`size_parts`] 提供——上游的 `data-[size=lg]:size-10` /
//!   `data-[size=sm]:size-6` 数据档位简化为直接传 Tailwind 尺寸 class
//!   （默认 `size-8`，`size-9`/`size-10` 等原样透传）。
//! - Image：`aspect-square size-full object-cover`（上游 `aspect-square size-full`
//!   补裁剪语义 `object-cover`）。
//! - Fallback：`flex size-full items-center justify-center rounded-full bg-muted
//!   text-sm text-muted-foreground`（上游 `group-data-[size=sm]/avatar:text-xs`
//!   随数据档位简化一并省去）。
//!
//! 与上游 Radix 版的差别：Radix 的 AvatarImage 在图片加载失败时自动回落到
//! Fallback；这里是纯展示组件——有 `src` 只渲染 img，无 `src` 渲染 Fallback，
//! 加载失败回落行为未实现。

use dioxus::prelude::*;

/// Root 尺寸段：`None` 回落 shadcn 默认档 `"size-8"`，`Some(s)` 原样透传
/// （调用方传 `"size-9"`/`"size-10"` 等 Tailwind 尺寸 class）。
///
/// 透传语义下返回值借用入参生命周期（入参为字面量时即是 `&'static str`）；
/// 抽成纯函数供契约测试锁定默认档，组件内部也用它拼 Root class。
pub fn size_parts(size: Option<&str>) -> &str {
    size.unwrap_or("size-8")
}

/// Fallback 显示的首字符：取 `name` 的第一个 Unicode 字符，空名回落 `'?'`。
///
/// 抽成纯函数供契约测试锁定回落行为。
pub fn fallback_char(name: &str) -> char {
    name.chars().next().unwrap_or('?')
}

/// shadcn 风格圆形头像。
///
/// - `name`：显示名——作为 img 的 `alt`；无图时取首字符（[`fallback_char`]）
///   作 Fallback 文本。
/// - `src`：头像图片地址；`None` 渲染 Fallback 首字符。
/// - `size`：Root 尺寸 Tailwind class（`"size-8"`/`"size-9"`/`"size-10"`…）；
///   `None` 用 shadcn 默认档 `size-8`（[`size_parts`]）。
/// - `attributes`：透传到 Root span 的全局属性（id、data-* 等）。
#[component]
pub fn Avatar(
    name: String,
    #[props(default)] src: Option<String>,
    #[props(default)] size: Option<String>,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let size_class = size_parts(size.as_deref());
    let fallback = fallback_char(&name);

    rsx! {
        span {
            "data-slot": "avatar",
            class: "relative flex {size_class} shrink-0 overflow-hidden rounded-full select-none",
            ..attributes,
            if let Some(src) = src {
                img {
                    "data-slot": "avatar-image",
                    class: "aspect-square size-full object-cover",
                    src: "{src}",
                    alt: "{name}",
                }
            } else {
                span {
                    "data-slot": "avatar-fallback",
                    class: "flex size-full items-center justify-center rounded-full bg-muted text-sm text-muted-foreground",
                    "{fallback}"
                }
            }
        }
    }
}
