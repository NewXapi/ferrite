#![recursion_limit = "512"]

//! 客户端入口。wasm32 编译，由 `HydrationScripts` 在浏览器里启动，
//! 接管 SSR 渲染出的 DOM，之后交互全在客户端。

mod app;
mod pages;
mod ui;
mod users;
mod wire;

/// 只在 csr feature 下编。SSR 二进制不包含 wasm-bindgen 依赖。
/// `App` 直接走模块路径引用：它只被本函数用到，顶层 `use` 在非 csr 构建下会成为
/// unused import（clippy `-D warnings` 直接判 error）。
#[cfg(feature = "csr")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
