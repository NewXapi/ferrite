//! P0 路由解析回归（规格：todo/gateway-resilience.md P0）。
//!
//! 根因结论（2026-09-13 钉死）：路由解析代码无 bug。#151 冒烟的 404 `no_route`
//! 是运维层三连击，逐条实锤于旧会话轨迹：
//! 1. 第一份冒烟 TOML 给 `[[channels]]` 写了从未存在的 `keys = [...]` 字段
//!    （合法字段是 `api_key`）→ `missing field api_key`，gateway 启动即失败；
//! 2. 修好字段后 `listen = 127.0.0.1:3211` 撞上共享 dev 后端（`Address already
//!    in use`，ss 实锤监听方为共享 apps/api）——共享后端没有 route units，
//!    对外吐的正是同款 `404 no_route`（`gateway_pipeline::router.rs` 的
//!    NoRoute 映射，两个 app 共用）；
//! 3. `pgrep -f 'target/debug/gateway' | head -1` 在多个残留 gateway 进程里
//!    误中非目标进程发 HUP/kill，最终 `:3299` 的 404 来自一个仍带旧空配置的
//!    残留实例（mock 代理/上游日志全空 = 请求从未到达）。
//!
//! 本文件钉住「配置齐全的最小全链路必须 200」，防止上述任一环节回归成
//! 「代码坏了」的错觉。
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// 起本地 mock OpenAI 上游：绑 `127.0.0.1:0`，每个连接回固定
/// `chat.completion` JSON，并置位命中标记。零外网，进程内自足。
///
/// 返回 `(端口, 命中标记)`——命中标记用于断言「回包确实走过上游」，
/// 而非路由层凭空造出 200。
fn spawn_mock_upstream() -> (u16, Arc<AtomicBool>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock upstream");
    let port = listener.local_addr().expect("local_addr").port();
    let hit = Arc::new(AtomicBool::new(false));
    let listener_hit = Arc::clone(&hit);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut sock) = stream else { continue };
            let thread_hit = Arc::clone(&listener_hit);
            std::thread::spawn(move || {
                // 读掉请求头与请求体（不解析，mock 只关心有没有请求进来）。
                let mut buf = [0u8; 8192];
                let _ = sock.read(&mut buf);
                let body = r#"{"id":"mock-1","object":"chat.completion","model":"mock-gpt","choices":[{"index":0,"message":{"role":"assistant","content":"pong"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = sock.write_all(resp.as_bytes());
                thread_hit.store(true, Ordering::SeqCst);
            });
        }
    });
    (port, hit)
}

/// 把 TOML 写临时文件后走真实 `GatewayConfig::load`（与 egress_wiring 同款）。
fn load_toml(body: &str) -> gateway::config::GatewayConfig {
    let path = std::env::temp_dir().join(format!("ferrite-p0-{}.toml", std::process::id()));
    let mut f = std::fs::File::create(&path).expect("create temp config");
    f.write_all(body.as_bytes()).expect("write temp config");
    let cfg = gateway::config::GatewayConfig::load(&path).expect("load config");
    let _ = std::fs::remove_file(&path);
    cfg
}

#[tokio::test]
async fn p0_mock_upstream_chat_completion_resolves_200() {
    let (upstream_port, upstream_hit) = spawn_mock_upstream();
    let cfg = load_toml(&format!(
        r#"
[[keys]]
key = "sk-p0-test"
name = "p0"
group = "default"

[[channels]]
name = "mock-up"
provider_type = "openai"
base_url = "http://127.0.0.1:{upstream_port}"
api_key = "mock-secret"
models = ["mock-gpt"]
"#,
    ));

    let app = gateway::build_app(&cfg);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind gateway");
    let addr = listener.local_addr().expect("gateway addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve gateway");
    });

    // 快照侧自检：同一份配置构造的路由快照里 (default, mock-gpt) 必须有候选。
    // 若这里为空，404 的根因在「配置 → 快照」装配；非空则根因在运行期链路。
    let snapshot = gateway::load_snapshot(&cfg);
    let cands = dispatch::candidates_from_snapshot(&snapshot.units, "default", "mock-gpt");
    assert!(
        !cands.is_empty(),
        "配置 → 快照装配缺失候选：units={:?}",
        snapshot.units
    );

    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "model": "mock-gpt",
        "messages": [{"role": "user", "content": "ping"}]
    });
    let resp = client
        .post(format!("http://{addr}/v1/chat/completions"))
        .header("content-type", "application/json")
        .bearer_auth("sk-p0-test")
        .body(serde_json::to_vec(&payload).expect("serialize payload"))
        .send()
        .await
        .expect("send request");
    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    assert_eq!(
        status,
        reqwest::StatusCode::OK,
        "P0-1 配置齐全的最小全链路必须 200，got {status}: {body_text}"
    );
    assert!(
        upstream_hit.load(Ordering::SeqCst),
        "mock 上游必须收到请求（200 不能是路由层幻觉）"
    );

    server.abort();
}

/// P0-2（P1-B 落地后的回归锚点）：双渠道同模型，渠道相关 4xx 降层到第二渠道。
///
/// 实现要点（TODO(#158)，依赖 P1-B `FatalButSwitchable`）：
/// 1. 两个本地 mock 上游：A 对 `POST /v1/chat/completions` 回 404（渠道相关
///    4xx——换渠道可能成立），B 回固定 chat.completion JSON 200；
/// 2. `[[channels]]` 两段各指其一、priority 让 A 先被选中、models 同 mock-gpt；
/// 3. build_app → 请求 → 断言：客户端最终 200（B 的回包）、A 与 B 各收到一次
///    请求（证明发生了降层换候选，而非 A 根本不可达），且 A 的健康不被
///    该类失败污染（Neutral 语义）。400/413/422 请求相关 4xx 的单候选透传
///    形状另见 dispatch/tests/failure_scope.rs。
#[tokio::test]
#[ignore = "TODO(#158): P0 诊断，骨架未实现"]
async fn p0_channel_scoped_404_downgrades_to_second_channel() {
    todo!()
}
