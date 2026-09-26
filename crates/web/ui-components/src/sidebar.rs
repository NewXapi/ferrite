//! Sidebar — shadcn new-york-v4 风格侧边栏（极简 offcanvas 版）。
//!
//! class 契约逐字取自 [shadcn ui/sidebar.tsx]，data-slot / data-sidebar / data-state
//! 属性照抄；原版依赖的 CSS 变量宽高（`w-(--sidebar-width)`）与 Radix context 换成
//! Dioxus 直写：`open` 由调用方受控传入，开合用 data-state + transform class 切换
//! （见 [`state_parts`]）。
//!
//! 已知取舍（极简 offcanvas 版）：
//! - 不做 icon 折叠（collapsible=icon）、rail 与 tooltip，`group-data-[collapsible=icon]:*`
//!   段照抄上游保留（当前无分组容器命中，解析为不生效的死 class，便于同步上游零 diff）。
//! - 桌面（md+）常驻渲染，关闭态靠 `-translate-x-full` 滑出视口；移动端 `hidden`
//!   （上游的移动端走 Sheet 分支，本版暂不实现，需要时由调用方直接用 Sheet）。
//!
//! [shadcn ui/sidebar.tsx]: https://github.com/shadcn-ui/ui

use dioxus::core::AttributeValue;
use dioxus::prelude::*;

/// shadcn ui/sidebar.tsx 根容器 class（offcanvas 简化版：`sidebar-container` 的 fixed
/// 定位段与 `sidebar-inner` 的 bg-sidebar 段合并，宽度直写 `w-72`，token 类逐字）。
const SIDEBAR_BASE_CLASS: &str = "fixed inset-y-0 left-0 z-40 hidden h-svh w-72 flex-col border-r bg-sidebar text-sidebar-foreground transition-transform duration-200 ease-linear md:flex";

/// shadcn ui/sidebar.tsx:340 SidebarHeader class（逐字）。
const HEADER_CLASS: &str = "flex flex-col gap-2 p-2";

/// shadcn ui/sidebar.tsx:377 SidebarContent class（逐字）。
const CONTENT_CLASS: &str = "flex min-h-0 flex-1 flex-col gap-2 overflow-auto group-data-[collapsible=icon]:overflow-hidden";

/// shadcn ui/sidebar.tsx:350 SidebarFooter class（逐字）。
const FOOTER_CLASS: &str = "flex flex-col gap-2 p-2";

/// shadcn ui/sidebar.tsx:390 SidebarGroup class（逐字）。
const GROUP_CLASS: &str = "relative flex w-full min-w-0 flex-col p-2";

/// shadcn ui/sidebar.tsx:408-409 SidebarGroupLabel class（两段 cn 拼接，逐字）。
const GROUP_LABEL_CLASS: &str = "flex h-8 shrink-0 items-center rounded-md px-2 text-xs font-medium text-sidebar-foreground/70 ring-sidebar-ring outline-hidden transition-[margin,opacity] duration-200 ease-linear focus-visible:ring-2 [&>svg]:size-4 [&>svg]:shrink-0 group-data-[collapsible=icon]:-mt-8 group-data-[collapsible=icon]:opacity-0";

/// shadcn ui/sidebar.tsx:459 SidebarMenu class（逐字）。
const MENU_CLASS: &str = "flex w-full min-w-0 flex-col gap-1";

/// shadcn ui/sidebar.tsx:470 SidebarMenuItem class（逐字）。
const MENU_ITEM_CLASS: &str = "group/menu-item relative";

/// shadcn ui/sidebar.tsx:477 sidebarMenuButtonVariants 核心段（variant="default"、
/// size="default" 固定；极简取舍去掉 peer/ring/disabled/aria-disabled/state=open 段，
/// 保留 hover/active/data-[active] 与 svg/span 约束段，逐字拼接）。
const MENU_BUTTON_CLASS: &str = "flex w-full items-center gap-2 overflow-hidden rounded-md p-2 text-left text-sm hover:bg-sidebar-accent hover:text-sidebar-accent-foreground active:bg-sidebar-accent data-[active=true]:bg-sidebar-accent data-[active=true]:font-medium data-[active=true]:text-sidebar-accent-foreground [&>span:last-child]:truncate [&>svg]:size-4 [&>svg]:shrink-0";

/// Sidebar 开合 → (data-state 值, transform class)。
///
/// 上游靠 `group-data-[collapsible=offcanvas]` 把容器平移出视口；本版受控 `open`
/// 直写：开启 `translate-x-0`（就位），关闭 `-translate-x-full`（滑出左缘）。
/// 抽成纯函数供契约测试锁定视觉契约。
pub fn state_parts(open: bool) -> (&'static str, &'static str) {
    if open {
        ("open", "translate-x-0")
    } else {
        ("closed", "-translate-x-full")
    }
}

/// active → (data-active 值, SidebarMenuButton class)。
///
/// data-active 取 React `data-active={isActive}` 的字符串化结果（"true"/"false"，
/// 与 shadcn 属性选择器 `data-[active=true]` 对齐）；class 两态同串，状态样式全部
/// 走 `data-[active=true]` 属性选择器。抽成纯函数供契约测试锁定视觉契约。
pub fn menu_button_parts(active: bool) -> (&'static str, &'static str) {
    let key = if active { "true" } else { "false" };
    (key, MENU_BUTTON_CLASS)
}

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

/// shadcn new-york-v4 风格侧边栏（offcanvas 受控版）。
///
/// ```ignore
/// let mut open = use_signal(|| true);
/// Sidebar {
///     open: open(),
///     onclick: move |_| open.toggle(), // 自行选触发器控制开合
///     SidebarHeader { "应用名" }
///     SidebarContent {
///         SidebarGroup {
///             SidebarGroupLabel { "导航" }
///             SidebarMenu {
///                 for item in items {
///                     SidebarMenuItem {
///                         SidebarMenuButton { active: item.is_active, "{item.label}" }
///                     }
///                 }
///             }
///         }
///     }
///     SidebarFooter { "用户区" }
/// }
/// ```
///
/// - `open`：受控开合；开启 `translate-x-0`，关闭 `-translate-x-full`（transition
///   由根 class 的 `transition-transform duration-200 ease-linear` 承担）。
/// - `attributes`：透传到根 `aside` 上；`class` 追加在基串之后。
#[component]
pub fn Sidebar(
    /// 受控开合状态：true 展开就位，false 平移出视口（仅 md+ 可见）。
    open: bool,
    /// 透传到根 `aside` 的属性；`class` 追加在 shadcn 基串之后。
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    /// 侧边栏内容：一般由 SidebarHeader / SidebarContent / SidebarFooter 组成。
    children: Element,
) -> Element {
    let (state, transform) = state_parts(open);
    let class = with_class(attributes, &format!("{SIDEBAR_BASE_CLASS} {transform}"));

    rsx! {
        aside {
            "data-slot": "sidebar",
            "data-state": "{state}",
            ..class,
            {children}
        }
    }
}

/// 侧边栏头部（shadcn ui/sidebar.tsx:335 的 SidebarHeader）。
///
/// 透传到 header 元素的属性；`class` 追加在 shadcn Header 基串之后。
#[component]
pub fn SidebarHeader(
    /// 透传到 header 元素的属性；`class` 追加在 shadcn 基串之后。
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let class = with_class(attributes, HEADER_CLASS);

    rsx! {
        div {
            "data-slot": "sidebar-header",
            "data-sidebar": "header",
            ..class,
            {children}
        }
    }
}

/// 侧边栏可滚动主体（shadcn ui/sidebar.tsx:371 的 SidebarContent）。
#[component]
pub fn SidebarContent(
    /// 透传到 content 元素的属性；`class` 追加在 shadcn 基串之后。
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let class = with_class(attributes, CONTENT_CLASS);

    rsx! {
        div {
            "data-slot": "sidebar-content",
            "data-sidebar": "content",
            ..class,
            {children}
        }
    }
}

/// 侧边栏底部（shadcn ui/sidebar.tsx:346 的 SidebarFooter）。
#[component]
pub fn SidebarFooter(
    /// 透传到 footer 元素的属性；`class` 追加在 shadcn 基串之后。
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let class = with_class(attributes, FOOTER_CLASS);

    rsx! {
        div {
            "data-slot": "sidebar-footer",
            "data-sidebar": "footer",
            ..class,
            {children}
        }
    }
}

/// 菜单分组容器（shadcn ui/sidebar.tsx:385 的 SidebarGroup）。
#[component]
pub fn SidebarGroup(
    /// 透传到 group 元素的属性；`class` 追加在 shadcn 基串之后。
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let class = with_class(attributes, GROUP_CLASS);

    rsx! {
        div {
            "data-slot": "sidebar-group",
            "data-sidebar": "group",
            ..class,
            {children}
        }
    }
}

/// 分组标签（shadcn ui/sidebar.tsx:396 的 SidebarGroupLabel；`asChild` 不支持，
/// 固定渲染 `div`）。
#[component]
pub fn SidebarGroupLabel(
    /// 透传到 label 元素的属性；`class` 追加在 shadcn 基串之后。
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let class = with_class(attributes, GROUP_LABEL_CLASS);

    rsx! {
        div {
            "data-slot": "sidebar-group-label",
            "data-sidebar": "group-label",
            ..class,
            {children}
        }
    }
}

/// 菜单列表（shadcn ui/sidebar.tsx:454 的 SidebarMenu，`ul` 语义）。
#[component]
pub fn SidebarMenu(
    /// 透传到 `ul` 元素的属性；`class` 追加在 shadcn 基串之后。
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let class = with_class(attributes, MENU_CLASS);

    rsx! {
        ul {
            "data-slot": "sidebar-menu",
            "data-sidebar": "menu",
            ..class,
            {children}
        }
    }
}

/// 菜单条目容器（shadcn ui/sidebar.tsx:465 的 SidebarMenuItem，`li` 语义）。
#[component]
pub fn SidebarMenuItem(
    /// 透传到 `li` 元素的属性；`class` 追加在 shadcn 基串之后。
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let class = with_class(attributes, MENU_ITEM_CLASS);

    rsx! {
        li {
            "data-slot": "sidebar-menu-item",
            "data-sidebar": "menu-item",
            ..class,
            {children}
        }
    }
}

/// 菜单按钮（shadcn ui/sidebar.tsx:498 的 SidebarMenuButton；variant/size 固定
/// default，tooltip 不支持——icon 折叠态本版就不存在）。
///
/// 激活样式走 `data-active="true"` 属性选择器（见 [`menu_button_parts`]）；点击
/// 行为由调用方在 `attributes` 里传 `onclick` 承接。
#[component]
pub fn SidebarMenuButton(
    /// 是否激活：渲染 `data-active="true"`，命中 bg-accent / font-medium 段。
    active: bool,
    /// 透传到 `button` 元素的属性；`class` 追加在 shadcn 基串之后。
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let (data_active, base) = menu_button_parts(active);
    let class = with_class(attributes, base);

    rsx! {
        button {
            "data-slot": "sidebar-menu-button",
            "data-sidebar": "menu-button",
            "data-size": "default",
            "data-active": "{data_active}",
            ..class,
            {children}
        }
    }
}
