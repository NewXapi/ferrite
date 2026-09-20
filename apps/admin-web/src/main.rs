//! Web entry. Mounts the original console shell.

// 不用 manganis 的 asset!() 内联：Firefox 下内联注入会阻塞主线程导致白屏
// （wasm 启动后 DOM 永不挂载，实测 wasm 编译/实例化均成功、app 的 /api/*
// 请求能发出、但 DOM 零挂载）。改普通 <link>，由浏览器异步拉取。
use admin_web::RootApp;
use dioxus::prelude::*;

fn main() {
    // 401 静默刷新 + 过期清登录态的接线必须在首帧前注册。
    admin_web::init_auth();
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: "/assets/tailwind.out.css" }
        // 全局 toast 出口：组件内 ui::components::toast::toast() 触发。
        ui::components::toast::Toaster {}
        RootApp {}
    }
}
