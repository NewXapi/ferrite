//! Leptos SSR 试用应用：把服务端进程自身的资源占用渲染到页面上，
//! 用来直观看 leptos + axum 的 dev 期开销。读 /proc 零依赖。

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::LazyLock;
use std::time::Instant;

/// 进程启动时刻，算运行时长用。
static START: LazyLock<Instant> = LazyLock::new(Instant::now);

/// 页面外壳：hydration 脚本指向的 /pkg/leptos-web.js 在本试用里不存在
/// （SSR-only，没编 hydrate 包），浏览器控制台会有一条 404，不影响渲染。
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="zh">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <title>"Ferrite · Leptos SSR 试用"</title>
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <header>
            <h1>"Ferrite — Leptos SSR 试用"</h1>
        </header>
        <main>
            <HomePage />
        </main>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let stats = read_stats();

    view! {
        <h2>"服务端资源占用（每次刷新重读 /proc）"</h2>
        <ul>
            <li>{format!("进程 RSS: {:.1} MB", stats.rss_mb)}</li>
            <li>{format!("系统负载(1min): {}", stats.loadavg)}</li>
            <li>{format!("已运行: {:.0}s", stats.uptime_s)}</li>
        </ul>
        <p>
            "同一个数字也能经 server function 拿到（POST 端点，可直接 curl）：" " "
            <code>"/api/server_stats&lt;hash&gt;"</code>
        </p>
    }
}

/// server function：POST 端点，路径带编译期哈希后缀，浏览器和 curl 都能直接打。
#[server]
pub async fn server_stats() -> Result<Stats, ServerFnError> {
    Ok(read_stats())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub rss_mb: f64,
    pub loadavg: String,
    pub uptime_s: f64,
}

fn read_stats() -> Stats {
    let start = *START;
    Stats {
        rss_mb: read_rss_mb(),
        loadavg: read_loadavg(),
        uptime_s: start.elapsed().as_secs_f64(),
    }
}

/// /proc/self/status 的 VmRSS 行，单位是 kB。
fn read_rss_mb() -> f64 {
    match fs::read_to_string("/proc/self/status") {
        Ok(text) => text
            .lines()
            .find(|line| line.starts_with("VmRSS:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|kb| kb.parse::<f64>().ok())
            .map(|kb| kb / 1024.0)
            .unwrap_or(0.0),
        Err(_) => 0.0,
    }
}

/// /proc/loadavg 第一个字段。
fn read_loadavg() -> String {
    fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|text| text.split_whitespace().next().map(str::to_string))
        .unwrap_or_else(|| "?".to_string())
}
