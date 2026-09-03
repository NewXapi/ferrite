# `gateway-protocol`

## 文件

- `src/lib.rs` — 公开 codec、SSE 与错误类型。
- `src/codec/openai.rs` — OpenAI 请求和响应编解码。
- `src/codec/claude.rs` — Claude Messages 编解码。
- `src/codec/gemini.rs` — Gemini GenerateContent 编解码。
- `src/sse.rs` — 扫描 SSE 帧、keepalive 和完成信号。
- `src/error.rs` — 规范化上游错误。

