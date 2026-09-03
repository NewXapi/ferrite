# `gateway-protocol`

## 目录

```text
src/
├── lib.rs
├── error.rs
├── sse.rs
└── codec/
    ├── openai.rs
    ├── claude.rs
    └── gemini.rs
```

## 要实现

- OpenAI、Claude、Gemini 请求响应编解码。
- SSE 帧扫描。
- NormalizedError。
