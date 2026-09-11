//! PR2a adopting 组件（dropdown_menu / switch / skeleton）的 shadcn class 契约测试：
//! 组件抽出的 pub 纯映射/常量必须与 shadcn new-york-v4 源码逐字一致
//! （ui/dropdown-menu.tsx:44/76/157/172、ui/switch.tsx:19/27、ui/skeleton.tsx）。
//!
//! 为什么逐字断言：与 `shadcn_class_contract.rs` 同理，这些串是组件的**视觉契约**——
//! 任何一串 class 的漂移都是一次用户可见的样式回归。改 class 串的唯一合法理由是
//! 「同步 shadcn 上游新版本」，此时本测试的期望值应随上游 diff 一起更新，
//! 并在 PR 里贴出上游对照。

use ui_components::components::dropdown_menu::{
    CONTENT_CLASS, ITEM_CLASS, LABEL_CLASS, SEPARATOR_CLASS, content_parts,
};
use ui_components::components::skeleton::SKELETON_CLASS;
use ui_components::components::switch::{THUMB_CLASS, TRACK_CLASS, state_parts};

#[test]
fn switch_state_parts_match_shadcn() {
    // checked → (data-state, track class, thumb class)；track/thumb 串即
    // ui/switch.tsx:19/27，两态同串（状态样式走 data-[state=…] 属性选择器）。
    assert_eq!(
        state_parts(true),
        ("checked", TRACK_CLASS, THUMB_CLASS),
        "checked 态漂移"
    );
    assert_eq!(
        state_parts(false),
        ("unchecked", TRACK_CLASS, THUMB_CLASS),
        "unchecked 态漂移"
    );
}

#[test]
fn switch_track_class_matches_shadcn() {
    // ui/switch.tsx:19 track（size 固定 default）。
    assert_eq!(
        TRACK_CLASS,
        "peer group/switch inline-flex shrink-0 items-center rounded-full border border-transparent shadow-xs transition-all outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 data-[size=default]:h-[1.15rem] data-[size=default]:w-8 data-[size=sm]:h-3.5 data-[size=sm]:w-6 data-[state=checked]:bg-primary data-[state=unchecked]:bg-input dark:data-[state=unchecked]:bg-input/80",
        "track class 漂移"
    );
}

#[test]
fn switch_thumb_class_matches_shadcn() {
    // ui/switch.tsx:27 thumb；translate-x 由 data-state 驱动，照抄 shadcn 写法。
    assert_eq!(
        THUMB_CLASS,
        "pointer-events-none block rounded-full bg-background ring-0 transition-transform group-data-[size=default]/switch:size-4 group-data-[size=sm]/switch:size-3 data-[state=checked]:translate-x-[calc(100%-2px)] data-[state=unchecked]:translate-x-0 dark:data-[state=checked]:bg-primary-foreground dark:data-[state=unchecked]:bg-foreground",
        "thumb class 漂移"
    );
}

#[test]
fn dropdown_menu_content_parts_match_shadcn() {
    // content_parts 的两态键名 + ui/dropdown-menu.tsx:44 Content class（逐字，
    // 含 data-[state] 动画 class 与 Radix 自定义属性引用——后者在本实现解析为空，
    // 保留以便上游同步零 diff）。
    let (open_state, open_class) = content_parts(true);
    let (closed_state, closed_class) = content_parts(false);
    assert_eq!(open_state, "open");
    assert_eq!(closed_state, "closed");
    assert_eq!(open_class, closed_class, "两态同串");
    assert_eq!(
        open_class,
        "z-50 max-h-(--radix-dropdown-menu-content-available-height) min-w-[8rem] origin-(--radix-dropdown-menu-content-transform-origin) overflow-x-hidden overflow-y-auto rounded-md border bg-popover p-1 text-popover-foreground shadow-md data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95",
        "content class 漂移"
    );
    assert_eq!(open_class, CONTENT_CLASS, "content_parts 与常量一致");
}

#[test]
fn dropdown_menu_item_class_matches_shadcn() {
    // ui/dropdown-menu.tsx:76 Item class（逐字）。
    assert_eq!(
        ITEM_CLASS,
        "relative flex cursor-default items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-hidden select-none focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50 data-[inset]:pl-8 data-[variant=destructive]:text-destructive data-[variant=destructive]:focus:bg-destructive/10 data-[variant=destructive]:focus:text-destructive dark:data-[variant=destructive]:focus:bg-destructive/20 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4 [&_svg:not([class*='text-'])]:text-muted-foreground data-[variant=destructive]:*:[svg]:text-destructive!",
        "item class 漂移"
    );
}

#[test]
fn dropdown_menu_label_and_separator_classes_match_shadcn() {
    // ui/dropdown-menu.tsx:157 Label、:172 Separator（逐字）。
    assert_eq!(
        LABEL_CLASS, "px-2 py-1.5 text-sm font-medium data-[inset]:pl-8",
        "label class 漂移"
    );
    assert_eq!(
        SEPARATOR_CLASS, "-mx-1 my-1 h-px bg-border",
        "separator class 漂移"
    );
}

#[test]
fn skeleton_class_matches_shadcn() {
    // ui/skeleton.tsx 基础 class（逐字）。
    assert_eq!(SKELETON_CLASS, "animate-pulse rounded-md bg-accent");
}
