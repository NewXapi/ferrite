//! 服务端入口。编 SSR 二进制，吐首屏 HTML + 注册 server function 路由。
//!
//! 模块公开：SSR 二进制与 wasm 侧共用同一套源码，`ui` 的 `pub use` 出口
//! 只有模块可见时才算被引用，否则 bin 里会报 unused_imports。

pub mod app;
pub mod pages;
pub mod ui;
pub mod users;
pub mod wire;

use std::sync::Arc;

use app::{App, shell};
use axum::Router;
use leptos::config::get_configuration;
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, generate_route_list};
use tokio::sync::Mutex;
use users::PageState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 读同目录 Cargo.toml 的 [package.metadata.leptos]，env 变量可覆盖。
    let conf = get_configuration(Some("Cargo.toml"))?;
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    // 用户列表活在进程里：SSR 首屏读它，客户端通过 server function 改它。
    let state = Arc::new(Mutex::new(PageState::fresh()));

    let app = Router::new()
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            {
                let state = state.clone();
                move || provide_context(state.clone())
            },
            {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("leptos-web listening on http://{addr}");
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}
