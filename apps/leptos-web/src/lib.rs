//! 客户端入口。wasm32 编译，由 `HydrationScripts` 在浏览器里启动，
//! 接管 SSR 渲染出的 DOM，之后交互全在客户端。

mod app;
mod users;

use app::App;

/// 只在 csr feature 下编。SSR 二进制不包含 wasm-bindgen 依赖。
#[cfg(feature = "csr")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
