//! SIGHUP 热更新回归测试：起真实进程、改配置、发 SIGHUP、断言 reload 完成。
//!
//! 为什么需要进程级测试：`hot_reload.rs` 只测 `Dispatcher::set_snapshot`，
//! 抓不到 `serve()` 里的死锁——那个 bug 是 `axum::serve` 被 `select!` 包着而
//! 没有 `with_graceful_shutdown`，`shutdown_tx.send(true)` 没有接收者，
//! main 的 `handle.await` 永久阻塞，reload 永不完成且端口一直被占。
//!
//! 运行：cargo test -p gateway --test reload_smoke -- --ignored

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

const PORT: u16 = 38472;

fn write_config(dir: &std::path::Path, cooldown_threshold: u32, max_attempts: u32) {
    let body = format!(
        "listen = \"127.0.0.1:{PORT}\"\n\
         log_level = \"info\"\n\
         \n[dispatch]\ncooldown_threshold = {cooldown_threshold}\n\
         \n[retry]\nmax_attempts = {max_attempts}\n"
    );
    std::fs::write(dir.join("config/config.toml"), body).expect("write config");
}

/// 后台线程逐行读进程日志推进 channel。
///
/// 日志走 stdout（见 `observability::init_tracing` 的 fmt layer），不是 stderr。
/// 必须用线程 + channel：`read_line` 在无输出时永久阻塞，超时检查根本轮不到，
/// 进程一旦不打日志测试就挂死而不是失败。
fn spawn_log_reader(out: std::process::ChildStdout) -> Receiver<String> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    rx
}

/// 等日志出现含 `needle` 的行；超时返回已读到的内容便于诊断。
fn wait_for_log(rx: &Receiver<String>, needle: &str, timeout: Duration) -> Result<String, String> {
    let deadline = Instant::now() + timeout;
    let mut seen = String::new();
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(format!("timeout waiting for {needle:?}; saw:\n{seen}"));
        }
        match rx.recv_timeout(left) {
            Ok(line) => {
                seen.push_str(&line);
                seen.push('\n');
                if line.contains(needle) {
                    return Ok(line);
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                return Err(format!("timeout waiting for {needle:?}; saw:\n{seen}"));
            }
            Err(RecvTimeoutError::Disconnected) => {
                return Err(format!("process output closed; saw:\n{seen}"));
            }
        }
    }
}

/// 发一个请求，返回状态码。无 token 时 AuthGate 应回 401 —— 证明 pipeline 活着。
fn probe() -> Result<u16, String> {
    let mut sock = TcpStream::connect(("127.0.0.1", PORT)).map_err(|e| format!("connect: {e}"))?;
    sock.set_read_timeout(Some(Duration::from_secs(5))).ok();
    let body = r#"{"model":"gpt-4o","messages":[]}"#;
    let req = format!(
        "POST /v1/chat/completions HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    );
    sock.write_all(req.as_bytes())
        .map_err(|e| format!("write: {e}"))?;
    let mut buf = Vec::new();
    let _ = sock.read_to_end(&mut buf);
    let head = String::from_utf8_lossy(&buf);
    head.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format!("bad response: {head:?}"))
}

/// SIGHUP 后 reload 必须在有界时间内完成，且新配置值出现在日志里。
///
/// 死锁时这个测试卡在 `wait_for_log("reload complete")` 超时。
#[test]
#[ignore = "spawns a real gateway process and binds a port"]
fn sighup_reload_completes_and_applies_new_values() {
    let dir = std::env::temp_dir().join(format!("ferrite-reload-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("config")).expect("mkdir");
    write_config(&dir, 2, 5);

    let bin = env!("CARGO_BIN_EXE_gateway");
    let mut child = Command::new(bin)
        .current_dir(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn gateway");
    let log = spawn_log_reader(child.stdout.take().expect("stdout piped"));

    // 启动日志带生效的配置值，而不是默认 5 / 3。
    let line = wait_for_log(&log, "gateway serving", Duration::from_secs(20))
        .expect("gateway should start");
    assert!(
        line.contains("cooldown_threshold=2") && line.contains("max_attempts=5"),
        "启动日志应含配置值，实际: {line}"
    );
    assert_eq!(probe().expect("probe"), 401, "无 token 应被 AuthGate 拦截");

    // 改配置后 SIGHUP；用 kill(1) 发信号，省一个 libc dev-dependency。
    write_config(&dir, 9, 2);
    let killed = Command::new("kill")
        .args(["-HUP", &child.id().to_string()])
        .status()
        .expect("run kill");
    assert!(killed.success(), "kill -HUP failed");

    let line = wait_for_log(&log, "reload complete", Duration::from_secs(20))
        .expect("reload 必须完成——卡住说明 serve 任务没响应停止信号");
    assert!(
        line.contains("cooldown_threshold=9") && line.contains("max_attempts=2"),
        "reload 日志应含新配置值，实际: {line}"
    );

    // 端口必须被重新绑定：旧任务若不释放 socket，新任务 bind 失败，服务就死了。
    wait_for_log(&log, "gateway serving", Duration::from_secs(20)).expect("reload 后应重新监听");
    assert_eq!(
        probe().expect("probe after reload"),
        401,
        "reload 后服务应仍可用"
    );

    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&dir);
}
