//! PR2d 契约测试：Avatar 的 Root 尺寸段映射与 Fallback 首字符回落。
//!
//! 为什么逐字断言：`size-8` 默认档与透传语义是 Avatar 的**视觉契约**——
//! shadcn new-york-v4 ui/avatar.tsx Root `size = "default"` 档的尺寸段就是
//! `size-8`；首字符回落决定无图头像的可见文本。改动的唯一合法理由是
//! 「同步 shadcn 上游新版本」，此时期望值应随上游 diff 一起更新。

use ui_components::components::avatar::{fallback_char, size_parts};

#[test]
fn avatar_size_parts_defaults_and_passthrough() {
    // shadcn ui/avatar.tsx Root：size = "default" 档的尺寸段是 size-8。
    assert_eq!(size_parts(None), "size-8");
    // 显式传入的尺寸段原样透传：上游 data-[size=lg/sm] 数据档位在本实现里
    // 简化为调用方直接传 Tailwind 尺寸 class，映射函数不得改写它。
    assert_eq!(size_parts(Some("size-9")), "size-9");
    assert_eq!(size_parts(Some("size-10")), "size-10");
    assert_eq!(size_parts(Some("size-6")), "size-6");
}

#[test]
fn avatar_fallback_char_first_char() {
    // 取 name 的第一个 Unicode 字符（多字节字符按整字符取，不按字节截断）。
    assert_eq!(fallback_char("Alice"), 'A');
    assert_eq!(fallback_char("张三"), '张');
    // 空名回落 '?'，保证 Fallback 不渲染出空文本节点。
    assert_eq!(fallback_char(""), '?');
}
