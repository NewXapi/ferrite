//! 页面共用的布局。组件优先用 singlestage（shadcn 风格），这里只补它没有的网格。

use leptos::prelude::*;

/// 卡片网格。web 5 栏，平板 3 栏，手机 1 栏。
#[component]
pub fn CardGrid(children: Children) -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5">
            {children()}
        </div>
    }
}
