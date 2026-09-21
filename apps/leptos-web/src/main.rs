mod app;

use app::{shell, App};
use axum::{routing::post, Router};
use leptos::config::get_configuration;
use leptos_axum::{generate_route_list, handle_server_fns, LeptosRoutes};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 读同目录 Cargo.toml 的 [package.metadata.leptos]，env 变量可覆盖。
    let conf = get_configuration(Some("Cargo.toml"))?;
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let app = Router::new()
        .route("/api/{*fn_name}", post(handle_server_fns))
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("leptos-web listening on http://{addr}");
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}
