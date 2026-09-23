//! 页面共用的布局与组件出口。组件优先用 singlestage（shadcn 风格），这里只补它没有的网格，
//! 并把页面高频引用的组件从 singlestage 统一转出。

use leptos::prelude::*;

/// singlestage 组件统一出口：页面写 `use crate::ui::{…}` 即可拿到
/// Button / Card / Dialog 三族及 Input、Label，不必关心 singlestage 的私有模块结构。
/// 部分子件当前仍从页面里的 `singlestage::*` 直取，可能显示为 crate 内未消费，
/// 故整体放行 unused_imports，防止 clippy `-D warnings` 误伤。
#[allow(unused_imports)]
pub use singlestage::{
    Button, Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle, Dialog,
    DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, DialogTrigger,
    Input, Label,
};

/// 卡片网格。web 5 栏，平板 3 栏，手机 1 栏。
#[component]
pub fn CardGrid(children: Children) -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5">
            {children()}
        </div>
    }
}
