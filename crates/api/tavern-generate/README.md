# `tavern-generate`

## src/lib.rs

- `GenerateConfig` — OpenAI 兼容上游地址。
- `GenerateState` — 当前用户目录、HTTP client 和上游配置。
- `router` — `POST /tavern/generate` 与 `GET /tavern/status`。
- `generate` — 读取当前用户 key，转发请求体，逐字节透传 SSE 响应。若请求体含 `_ferrite_agent_prompt_marker`(或 legacy 别名),先经 `harness-prompt::reject_unfinalized_snapshot` 校验(残留 `{{...}}` 或未置空 marker 一律 400),校验通过后摘除 marker 再转发;无 marker 的 payload 原样透传。

## 参考实现

- `/home/hathaway/projects/SillyTavern/src/util.js:709` — forwardFetchResponse。
- `/home/hathaway/projects/new-api/apps/api/relay/helper/stream_scanner.go:77` — StreamScannerHandler。
