//! Tavern web entry. Mounts the tavern shell.

// 不用 manganis 的 asset!() 内联：Firefox 下内联注入会阻塞主线程导致白屏
// （wasm 启动后 DOM 永不挂载）。改普通 <link>，由浏览器异步拉取。
use dioxus::prelude::*;
use tavern_web_app::TavernApp;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: "/assets/tailwind.out.css" }
        // 全局 toast 出口：组件内 ui::components::toast::toast() 触发。
        ui_components::components::toast::Toaster {}
        TavernApp {}
    }
}
