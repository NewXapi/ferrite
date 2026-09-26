//! Web entry. Mounts the original console shell.

use admin_web::RootApp;
use dioxus::prelude::*;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.out.css");

// Ainotation 标注工具 bundle，由 `bun run aino` 生成（源：ainotation-entry.ts）。
#[cfg(debug_assertions)]
const AINOTATION_JS: Asset = asset!("/assets/ainotation/ainotation.iife.js");

fn main() {
    // 401 静默刷新 + 过期清登录态的接线必须在首帧前注册。
    admin_web::init_auth();
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Ainotation 标注工具 bundle，由 `bun run aino` 生成（源：ainotation-entry.ts）。
    // 仅开发环境加载；release 构建自动排除。用法见 apps/admin-web/AINOTATION.md。
    #[cfg(debug_assertions)]
    let ainotation_js: Option<Asset> = Some(AINOTATION_JS);
    #[cfg(not(debug_assertions))]
    let ainotation_js: Option<Asset> = None;

    rsx! {
        document::Stylesheet { href: TAILWIND_CSS }
        // 全局 toast 出口：组件内 ui::toast::toast() 触发。
        ui::toast::Toaster {}
        {ainotation_js.map(|src| rsx! { document::Script { src } })}
        RootApp {}
    }
}
