//! Button — dsh（deepseek-harness）风格按钮基元。
//!
//! 视觉基准已由 shadcn new-york-v4 逐字串切换为 dsh 组件 CSS：几何与状态取自
//! dsh `packages/client/ui-*/src/Button.module.css:4-17`（md/sm 几何、disabled、
//! fill/hover）与 `PluginCard.module.css:37-40`（focus outline 惯例），映射与决策
//! 记录见 `todo/web-ui-reference/dsh-visual-spec.md` §3.1/§4.1。调用方传入的
//! `class` 由 [`with_class`] 追加到组件基串之后，其余属性原样透传。

use dioxus::core::AttributeValue;
use dioxus::prelude::*;

/// dsh 基准的按钮基础 class（Button.module.css:4-17 几何 + PluginCard.module.css:37-40 focus 惯例）。
///
/// gap-1（:8）、rounded-full（:10，dsh md/sm 圆角均为半高胶囊，rounded-full 随高度自适应）、
/// text-sm（fs14 :12）、font-medium（dsh base.css:1-3 wt510→500）；focus 由 ring 三段改为
/// outline 2px offset -2（D10）；transition-colors duration-150 为保留微过渡的拍板项（D11，
/// dsh 本体无 transition）；disabled:opacity-40（:19-22）；aria-invalid 的 ring 段同步降为 outline。
const BUTTON_BASE_CLASS: &str = "inline-flex shrink-0 items-center justify-center gap-1 rounded-full text-sm font-medium whitespace-nowrap transition-colors duration-150 outline-none focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring disabled:pointer-events-none disabled:opacity-40 aria-invalid:border-destructive aria-invalid:outline-destructive [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4";

#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Destructive,
    Outline,
    Ghost,
    Link,
}

impl ButtonVariant {
    /// 变体的稳定 slug（沿用旧 data-style 键名，供调用方做属性钩子）。
    pub fn class(&self) -> &'static str {
        match self {
            ButtonVariant::Primary => "primary",
            ButtonVariant::Secondary => "secondary",
            ButtonVariant::Destructive => "destructive",
            ButtonVariant::Outline => "outline",
            ButtonVariant::Ghost => "ghost",
            ButtonVariant::Link => "link",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum ButtonSize {
    Xs,
    Sm,
    #[default]
    Default,
    Lg,
    Icon,
    IconXs,
    IconSm,
    IconLg,
}

impl ButtonSize {
    /// 尺寸的稳定 slug（沿用旧 data-size 键名，供调用方做属性钩子）。
    pub fn class(&self) -> &'static str {
        match self {
            ButtonSize::Xs => "xs",
            ButtonSize::Sm => "sm",
            ButtonSize::Default => "default",
            ButtonSize::Lg => "lg",
            ButtonSize::Icon => "icon",
            ButtonSize::IconXs => "icon-xs",
            ButtonSize::IconSm => "icon-sm",
            ButtonSize::IconLg => "icon-lg",
        }
    }
}

/// 变体对应的 `data-variant` 键名与 class 串（键名沿用 shadcn slug，class 按 dsh 行为改写）。
///
/// dsh 行为出处：Button.module.css:38-71（fill/hover）、design-platform.css:278-279/:292/:288
/// （语义别名）。secondary hover 用 `--secondary-hover`（D5，dsh button-ghost-active-hover :279）；
/// ghost active 白 14% 取最近标准刻度 white/15（:51-53/:288）；outline 去 shadow/dark: 段，
/// dsh hover 只变底色（:56-63）；destructive 保留 shadcn 红底串不变（D9，dsh 无红底变体）。
/// 公开为组件视觉契约的一部分，供 `tests/` 契约测试与需要按变体取样的调用方使用。
pub fn variant_parts(variant: ButtonVariant) -> (&'static str, &'static str) {
    match variant {
        ButtonVariant::Primary => (
            "default",
            "bg-primary text-primary-foreground hover:bg-primary/90",
        ),
        ButtonVariant::Secondary => (
            "secondary",
            "bg-secondary text-secondary-foreground hover:bg-secondary-hover",
        ),
        ButtonVariant::Destructive => (
            "destructive",
            "bg-destructive text-white hover:bg-destructive/90 focus-visible:ring-destructive/20 dark:bg-destructive/60 dark:focus-visible:ring-destructive/40",
        ),
        ButtonVariant::Outline => ("outline", "border bg-transparent hover:bg-accent"),
        ButtonVariant::Ghost => ("ghost", "hover:bg-accent active:bg-white/15"),
        ButtonVariant::Link => ("link", "text-primary underline-offset-4 hover:underline"),
    }
}

/// 尺寸对应的 `data-size` 键名与 class 串（键名沿用 shadcn slug，class 按 dsh 几何改写）。
///
/// dsh 几何出处：Button.module.css:30-36（sm h28/pad 10px/fs12）、:16（default pad 14px
/// → px-3.5）、:28-29（icon-only 容器 28×28 → icon-sm size-7，D8）；xs/lg 为 ferrite
/// 独有档位（dsh 无 24px/38px 公用规格，D7），仅随基串调整。所有尺寸串不再带
/// rounded-md：基串已改 rounded-full，同属性 utility 的胜负由样式表顺序决定而非
/// class 串顺序，残留会随机覆盖胶囊（spec §2）。公开理由同 [`variant_parts`]。
pub fn size_parts(size: ButtonSize) -> (&'static str, &'static str) {
    match size {
        ButtonSize::Xs => (
            "xs",
            "h-6 gap-1 px-2 text-xs has-[>svg]:px-1.5 [&_svg:not([class*='size-'])]:size-3",
        ),
        ButtonSize::Sm => ("sm", "h-7 gap-1 px-2.5 text-xs has-[>svg]:px-2"),
        ButtonSize::Default => ("default", "h-9 px-3.5 py-2 has-[>svg]:px-3"),
        ButtonSize::Lg => ("lg", "h-10 px-6 has-[>svg]:px-4"),
        ButtonSize::Icon => ("icon", "size-9"),
        // icon-xs 键名沿用 shadcn data-size（ui/button.tsx:28）：size-6 方形 + svg 缩到
        // size-3，与我们多出的 IconXs 枚举一一对应，无需自造等价组合。
        ButtonSize::IconXs => (
            "icon-xs",
            "size-6 [&_svg:not([class*='size-'])]:size-3",
        ),
        ButtonSize::IconSm => ("icon-sm", "size-7"),
        ButtonSize::IconLg => ("icon-lg", "size-10"),
    }
}

/// 把调用方传入的 `class` 追加到组件基串之后，其余属性原样保留。
///
/// 取代 dxc 的 `dioxus_primitives::merge_attributes`：只摘出 `class` 属性做字符串
/// 拼接（先组件基串、后调用方串；Tailwind 语义下拼接顺序不影响命中），非 `class`
/// 属性原样透传，保证调用方传的 class 与其他属性都不丢。
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

/// dsh（deepseek-harness）风格按钮。
#[component]
pub fn Button(
    #[props(default)] variant: ButtonVariant,
    #[props(default)] size: ButtonSize,
    #[props(extends=GlobalAttributes)]
    #[props(extends=button)]
    attributes: Vec<Attribute>,
    onclick: Option<EventHandler<MouseEvent>>,
    onmousedown: Option<EventHandler<MouseEvent>>,
    onmouseup: Option<EventHandler<MouseEvent>>,
    onkeydown: Option<EventHandler<KeyboardEvent>>,
    children: Element,
) -> Element {
    let (variant_key, variant_class) = variant_parts(variant);
    let (size_key, size_class) = size_parts(size);
    let class = with_class(
        attributes,
        &format!("{BUTTON_BASE_CLASS} {variant_class} {size_class}"),
    );

    rsx! {
        button {
            "data-slot": "button",
            "data-variant": variant_key,
            "data-size": size_key,
            onclick: move |event| {
                if let Some(f) = &onclick {
                    f.call(event);
                }
            },
            onmousedown: move |event| {
                if let Some(f) = &onmousedown {
                    f.call(event);
                }
            },
            onmouseup: move |event| {
                if let Some(f) = &onmouseup {
                    f.call(event);
                }
            },
            onkeydown: move |event| {
                if let Some(f) = &onkeydown {
                    f.call(event);
                }
            },
            ..class,
            {children}
        }
    }
}
