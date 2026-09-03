# `gateway-forward`

## 文件

- `src/lib.rs` — 公开上游转发入口。
- `src/adapter.rs` — 根据渠道类型生成 URL、认证头和请求体。
- `src/egress.rs` — 发起上游 HTTP 请求。
- `src/pipeline.rs` — 把 RequestCtx 转成一次上游 attempt。
- `src/stream.rs` — 读取 SSE、记录首 token、提取 usage 并传给客户端。

