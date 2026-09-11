//! DropdownMenu — shadcn new-york-v4 风格下拉菜单。
//!
//! 视觉逐字取自 shadcn ui/dropdown-menu.tsx（Content/Item/Label/Separator 的 class
//! 与 data-slot / data-state 属性）；Radix 负责的交互部分换成 Dioxus 自研实现，
//! 不引入任何新依赖：
//!
//! - **开合**：内部 `use_signal` 管理 `open`；trigger 插槽外面包一层 span 承接
//!   click 后切换（无法向任意 Element 注入 onclick，包裹层是自研的最小妥协）。
//! - **点击面板外自动关闭**：挂载时用 `document::eval` 给 `window` 挂 click
//!   **捕获**监听，按「target 是否被根节点 `contains`」判断内外，根外点击回传
//!   关闭；卸载时 `use_drop` 拆掉监听（与 `scroll_spy` 的 guard 收尾模式一致）。
//! - **定位**：Content 是 `absolute` 面板，定位上下文由调用方提供 —— 外层包一个
//!   `relative` 容器，或用 `content_class` 传 `top-full left-0` 之类的定位 class。
//!
//! 已知取舍：`DropdownMenuItem` 不自动关闭（见其文档注释）；Content 打开即渲染、
//! 关闭即卸载，`data-[state=closed]` 退场动画 class 逐字保留但当前没有退场过程。

use std::sync::atomic::{AtomicUsize, Ordering};

use dioxus::core::AttributeValue;
use dioxus::prelude::*;

/// shadcn ui/dropdown-menu.tsx:44 Content class（逐字，含 data-[state] 动画 class）。
///
/// `max-h-(--radix-…)` / `origin-(--radix-…)` 引用的 Radix 自定义属性在本实现里
/// 不存在，解析为空值不生效；逐字保留以便同步上游时零 diff。
pub const CONTENT_CLASS: &str = "z-50 max-h-(--radix-dropdown-menu-content-available-height) min-w-[8rem] origin-(--radix-dropdown-menu-content-transform-origin) overflow-x-hidden overflow-y-auto rounded-md border bg-popover p-1 text-popover-foreground shadow-md data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95";

/// shadcn ui/dropdown-menu.tsx:76 Item class（逐字）。
pub const ITEM_CLASS: &str = "relative flex cursor-default items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-hidden select-none focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50 data-[inset]:pl-8 data-[variant=destructive]:text-destructive data-[variant=destructive]:focus:bg-destructive/10 data-[variant=destructive]:focus:text-destructive dark:data-[variant=destructive]:focus:bg-destructive/20 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4 [&_svg:not([class*='text-'])]:text-muted-foreground data-[variant=destructive]:*:[svg]:text-destructive!";

/// shadcn ui/dropdown-menu.tsx:157 Label class（逐字）。
pub const LABEL_CLASS: &str = "px-2 py-1.5 text-sm font-medium data-[inset]:pl-8";

/// shadcn ui/dropdown-menu.tsx:172 Separator class（逐字）。
pub const SEPARATOR_CLASS: &str = "-mx-1 my-1 h-px bg-border";

/// Content 的 data-state 与 class 串（ui/dropdown-menu.tsx:44，逐字）。
///
/// 两态 class 同串：状态样式全部走 `data-[state=…]` 属性选择器（shadcn 原样），
/// 状态键名跟随开关量。公开为组件视觉契约的一部分，供 `tests/` 契约测试断言。
pub fn content_parts(open: bool) -> (&'static str, &'static str) {
    if open {
        ("open", CONTENT_CLASS)
    } else {
        ("closed", CONTENT_CLASS)
    }
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

/// shadcn new-york-v4 风格下拉菜单。
///
/// ```ignore
/// DropdownMenu {
///     trigger: rsx! { Button { "选项" } },
///     content: rsx! {
///         DropdownMenuItem { onclick: move |_| {}, "重命名" }
///         DropdownMenuSeparator {}
///     },
///     content_class: Some("top-full left-0".into()),
/// }
/// ```
#[component]
pub fn DropdownMenu(
    /// 触发器插槽：一般传我们的 `Button`；插槽内的点击统一由本组件切换开合。
    trigger: Element,
    /// 面板内容插槽：一般由 DropdownMenuItem / DropdownMenuLabel / DropdownMenuSeparator 组成。
    content: Element,
    /// 追加到 Content 面板 class 之后的调用方 class（定位、宽度等，如 `top-full left-0 w-56`）。
    #[props(default)]
    content_class: Option<String>,
    /// 透传到根包裹 div 的属性；勿传 `id`（根 id 由组件分配，外点判断依赖它）。
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
) -> Element {
    let mut open = use_signal(|| false);

    // 根节点唯一 id：既是 JS 侧 contains 判断的锚点，也是监听守卫的命名空间；
    // 实例间互不干扰靠自增序号（wasm 单线程，Relaxed 足够）。
    let root_id = use_hook(|| {
        static SEQ: AtomicUsize = AtomicUsize::new(0);
        format!("dioxus-dropdown-{}", SEQ.fetch_add(1, Ordering::Relaxed))
    });

    // window click 捕获监听（挂载一次，target 包含判断）：根外点击回传 0 → 关闭。
    // eval 通道随组件卸载而死，recv 出错即退出循环；监听本体由 use_drop 拆除。
    use_hook({
        let root_id = root_id.clone();
        move || {
            let guard = format!("__dioxusDropdown_{}", root_id.replace('-', "_"));
            let js = format!(
                r#"
                {{
                    if (window.{guard} === undefined) {{
                        const fn = (event) => {{
                            const root = document.getElementById('{root_id}');
                            dioxus.send(root && root.contains(event.target) ? 1 : 0);
                        }};
                        window.addEventListener('click', fn, true);
                        window.{guard} = fn;
                    }}
                }}
                "#
            );
            spawn(async move {
                let mut ev = document::eval(&js);
                while let Ok(v) = ev.recv::<f64>().await {
                    // 0 = 点击发生在根节点之外
                    if v < 0.5 {
                        open.set(false);
                    }
                }
            });
        }
    });

    // 卸载时拆掉 JS 侧监听，防止对已死通道持续 dioxus.send（同 scroll_spy 的收尾约定）。
    use_drop({
        let guard = format!("__dioxusDropdown_{}", root_id.replace('-', "_"));
        move || {
            let _ = document::eval(&format!(
                r#"
                if (window.{guard}) {{
                    window.removeEventListener('click', window.{guard}, true);
                    delete window.{guard};
                }}
                "#
            ));
        }
    });

    // Content 面板：shadcn 基串 + 自研定位所需的 absolute + 调用方定位 class。
    let (content_state, content_base) = content_parts(true);
    let content_class = match content_class.as_deref().map(str::trim) {
        Some(extra) if !extra.is_empty() => format!("absolute {content_base} {extra}"),
        _ => format!("absolute {content_base}"),
    };

    rsx! {
        div {
            "data-slot": "dropdown-menu",
            "data-state": if open() { "open" } else { "closed" },
            // Escape 关闭：焦点在触发按钮/面板内按键可关（Radix 默认行为的自研等价）。
            onkeydown: move |event: KeyboardEvent| {
                if event.key() == Key::Escape {
                    open.set(false);
                }
            },
            ..attributes,
            span {
                "data-slot": "dropdown-menu-trigger",
                "aria-haspopup": "menu",
                "aria-expanded": "{open()}",
                onclick: move |_| open.toggle(),
                {trigger}
            }
            if open() {
                div {
                    "data-slot": "dropdown-menu-content",
                    "data-state": "{content_state}",
                    role: "menu",
                    class: "{content_class}",
                    {content}
                }
            }
        }
    }
}

/// 菜单项（shadcn ui/dropdown-menu.tsx:61 的 Item；`inset`/`variant` 固定 false/default）。
///
/// **不自动关闭**（极简方案）：点击后菜单保持打开。需要「选中即关」语义时，由调用方
/// 在自己的 `onclick` 里收尾（导航、提交后自行关闭或卸载整个菜单）。
#[component]
pub fn DropdownMenuItem(
    /// 点击回调；回调之后组件不做任何额外处理（含关闭）。
    onclick: EventHandler<MouseEvent>,
    /// 透传到 item 元素的属性；`class` 追加在 shadcn Item 基串之后。
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let class = with_class(attributes, ITEM_CLASS);

    rsx! {
        div {
            "data-slot": "dropdown-menu-item",
            "data-inset": "false",
            "data-variant": "default",
            role: "menuitem",
            tabindex: "-1",
            onclick: move |event| onclick.call(event),
            ..class,
            {children}
        }
    }
}

/// 菜单分组标签（shadcn ui/dropdown-menu.tsx:145 的 Label；`inset` 固定 false）。
#[component]
pub fn DropdownMenuLabel(
    /// 透传到 label 元素的属性；`class` 追加在 shadcn Label 基串之后。
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let class = with_class(attributes, LABEL_CLASS);

    rsx! {
        div {
            "data-slot": "dropdown-menu-label",
            "data-inset": "false",
            ..class,
            {children}
        }
    }
}

/// 菜单分隔线（shadcn ui/dropdown-menu.tsx:165 的 Separator）。
#[component]
pub fn DropdownMenuSeparator(
    /// 透传到分隔线元素的属性；`class` 追加在 shadcn Separator 基串之后。
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
) -> Element {
    let class = with_class(attributes, SEPARATOR_CLASS);

    rsx! {
        div {
            "data-slot": "dropdown-menu-separator",
            role: "separator",
            ..class,
        }
    }
}
