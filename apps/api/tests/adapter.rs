use api::adapter::{AdapterError, StreamResponse, ensure_stream_ok};
use std::io::{Read, Write};
use std::net::TcpListener;

/// 启动一个本地一次性 HTTP 服务器，返回固定状态码和 body。
/// 先读掉请求再响应，避免关闭时内核发 RST 导致客户端连接错误。
fn serve_once(status: u16, body: &'static str) -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let resp = format!(
                "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(resp.as_bytes());
        }
    });
    (format!("http://{addr}"), handle)
}

/// 2xx → ensure_stream_ok 返回 Ok
#[tokio::test]
async fn ensure_stream_ok_2xx_returns_ok() {
    let (url, _server) = serve_once(200, "ok");
    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await.unwrap();
    let sr = StreamResponse {
        status: resp.status().as_u16(),
        stream: resp,
    };

    let result = ensure_stream_ok(sr).await;
    assert!(result.is_ok());
}

/// 5xx → ensure_stream_ok 返回 Err(Upstream)
#[tokio::test]
async fn ensure_stream_ok_5xx_returns_error() {
    let (url, _server) = serve_once(500, "internal error");
    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await.unwrap();
    let sr = StreamResponse {
        status: resp.status().as_u16(),
        stream: resp,
    };

    let result = ensure_stream_ok(sr).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        AdapterError::Upstream(msg) => assert!(msg.contains("500")),
        _ => panic!("expected Upstream error"),
    }
}

/// 4xx → ensure_stream_ok 返回 Err(Upstream)
#[tokio::test]
async fn ensure_stream_ok_4xx_returns_error() {
    let (url, _server) = serve_once(404, "not found");
    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await.unwrap();
    let sr = StreamResponse {
        status: resp.status().as_u16(),
        stream: resp,
    };

    let result = ensure_stream_ok(sr).await;
    assert!(result.is_err());
}
