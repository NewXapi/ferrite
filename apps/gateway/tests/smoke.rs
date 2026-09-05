//! 端到端 smoke：spawn 真实 gateway 进程，用 std TCP 发 HTTP 验证全链路。
//!
//! 运行：cargo test -p gateway --test smoke -- --ignored

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

fn http_post(port: u16, path: &str, body: &str) -> Result<(u16, String), String> {
    let mut sock =
        TcpStream::connect(format!("127.0.0.1:{port}")).map_err(|e| format!("connect: {e}"))?;
    sock.set_read_timeout(Some(Duration::from_secs(5))).ok();
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    sock.write_all(req.as_bytes())
        .map_err(|e| format!("write: {e}"))?;
    let mut buf = Vec::new();
    let _ = sock.read_to_end(&mut buf);
    let resp = String::from_utf8_lossy(&buf);
    let status = resp
        .split("\r\n")
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse::<u16>().ok())
        .unwrap_or(0);
    Ok((status, resp.lines().last().unwrap_or("").to_string()))
}

fn wait_for_port(port: u16, timeout: Duration) -> bool {
    let start = std::time::Instant::now();
    loop {
        if TcpStream::connect(format!("127.0.0.1:{port}")).is_ok() {
            return true;
        }
        if start.elapsed() > timeout {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
#[ignore = "进程级 smoke：cargo test --test smoke -- --ignored"]
fn gateway_e2e_smoke() {
    // 定位 worktree 根：从测试可执行文件向上找到包含 config/ 的目录
    let exe = std::env::current_exe().expect("current_exe");
    let mut root = exe.as_path();
    loop {
        let candidate = root.join("config/config.toml.example");
        if candidate.exists() {
            break;
        }
        match root.parent() {
            Some(p) => root = p,
            None => panic!("找不到 config/ 目录"),
        }
    }

    let example = root.join("config/config.toml.example");
    let config = root.join("config/config.toml");
    std::fs::copy(&example, &config).expect("copy config.toml 应成功");

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_gateway"))
        .current_dir(root)
        .env("RUST_LOG", "warn")
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let port = 3000u16;

    if !wait_for_port(port, Duration::from_secs(15)) {
        let mut stderr = String::new();
        if let Some(mut out) = child.stderr.take() {
            let _ = out.read_to_string(&mut stderr);
        }
        let _ = child.wait();
        panic!("gateway 未 bind 端口 {port}（15s 超时）。stderr:\n{stderr}");
    }

    // smoke 1: 无 key → 401
    let (s, b) = http_post(port, "/v1/chat/completions", r#"{"model":"gpt-4o"}"#).unwrap();
    assert_eq!(s, 401, "无 key 应 401, got {s}: {b}");

    // smoke 2: 空快照 → 调度无候选（404）或仍 401（auth 先拒）
    let (s, b) = http_post(
        port,
        "/v1/chat/completions",
        r#"{"model":"gpt-4o","messages":[{"role":"user","content":"hi"}]}"#,
    )
    .unwrap();
    assert!(s == 401 || s == 404, "空快照 → 401/404, got {s}: {b}");

    // smoke 3: 流式不 panic
    let (s, _) = http_post(
        port,
        "/v1/chat/completions",
        r#"{"model":"gpt-4o","messages":[{"role":"user","content":"hi"}],"stream":true}"#,
    )
    .unwrap();
    assert!(s == 401 || s == 404, "stream smoke got {s}");

    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(&config);
}
