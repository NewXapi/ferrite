# `tavern-generate`

## src/lib.rs

- `GenerateConfig` — OpenAI 兼容上游地址。
- `GenerateState` — 当前用户目录、HTTP client 和上游配置。
- `router` — `POST /tavern/generate` 与 `GET /tavern/status`。
- `generate` — 读取当前用户 key，转发请求体，逐字节透传 SSE 响应。

## 参考实现

- `/home/hathaway/projects/SillyTavern/src/util.js:709` — forwardFetchResponse。
- `/home/hathaway/projects/new-api/apps/api/relay/helper/stream_scanner.go:77` — StreamScannerHandler。
