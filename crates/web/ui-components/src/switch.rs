//! Switch — shadcn new-york-v4 风格开关基元。
//!
//! track/thumb 的 class 与 data-slot / data-state / data-size 属性逐字取自
//! shadcn ui/switch.tsx；Radix 的受控开合换成 Dioxus 自研：组件是**受控**的，
//! 调用方持有 `checked` 状态，点击后组件回调 `on_checked_change(!checked)`，
//! 由调用方写回 prop 驱动 thumb 的 translate-x 位移（`data-[state=…]` 驱动，
//! shadcn 写法原样）。

use dioxus::core::AttributeValue;
use dioxus::prelude::*;

/// shadcn ui/switch.tsx:19 track class（逐字；size 固定 default）。
pub const TRACK_CLASS: &str = "peer group/switch inline-flex shrink-0 items-center rounded-full border border-transparent shadow-xs transition-all outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 data-[size=default]:h-[1.15rem] data-[size=default]:w-8 data-[size=sm]:h-3.5 data-[size=sm]:w-6 data-[state=checked]:bg-primary data-[state=unchecked]:bg-input dark:data-[state=unchecked]:bg-input/80";

/// shadcn ui/switch.tsx:27 thumb class（逐字；translate-x 由 data-state 驱动）。
pub const THUMB_CLASS: &str = "pointer-events-none block rounded-full bg-background ring-0 transition-transform group-data-[size=default]/switch:size-4 group-data-[size=sm]/switch:size-3 data-[state=checked]:translate-x-[calc(100%-2px)] data-[state=unchecked]:translate-x-0 dark:data-[state=checked]:bg-primary-foreground dark:data-[state=unchecked]:bg-foreground";

/// checked → (data-state, track class, thumb class) 三元组。
///
/// track/thumb class 两态同串：状态样式全部走 `data-[state=…]` 属性选择器
/// （shadcn 原样），data-state 同时写在 track 与 thumb 的属性上驱动位移。
/// 公开为组件视觉契约的一部分，供 `tests/` 契约测试断言。
pub fn state_parts(checked: bool) -> (&'static str, &'static str, &'static str) {
    match checked {
        true => ("checked", TRACK_CLASS, THUMB_CLASS),
        false => ("unchecked", TRACK_CLASS, THUMB_CLASS),
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

/// shadcn new-york-v4 风格开关（受控）。
///
/// ```ignore
/// let mut enabled = use_signal(|| false);
/// Switch { checked: enabled(), on_checked_change: move |v| enabled.set(v) }
/// ```
#[component]
pub fn Switch(
    /// 当前受控状态；点击只回调不内改，视觉位移随调用方写回后的重渲染发生。
    checked: bool,
    /// 状态切换回调：点击后收到 `!checked`，由调用方决定是否写回。
    on_checked_change: EventHandler<bool>,
    /// 透传到 button 元素的属性；`class` 追加在 shadcn track 基串之后。
    #[props(extends = GlobalAttributes)]
    #[props(extends = button)]
    attributes: Vec<Attribute>,
    /// 是否可交互：`None` 与 `Some(true)` 视为启用；`Some(false)` 渲染原生
    /// `disabled` 按钮，shadcn 的 `disabled:` 样式类随之生效（disabled 处理可选）。
    #[props(default)]
    enabled: Option<bool>,
) -> Element {
    let (state, track_class, thumb_class) = state_parts(checked);
    let class = with_class(attributes, track_class);
    let disabled = matches!(enabled, Some(false));

    rsx! {
        button {
            "data-slot": "switch",
            "data-size": "default",
            "data-state": state,
            role: "switch",
            "aria-checked": "{checked}",
            type: "button",
            disabled,
            onclick: move |_| on_checked_change.call(!checked),
            ..class,
            span {
                "data-slot": "switch-thumb",
                "data-state": state,
                class: "{thumb_class}",
            }
        }
    }
}
