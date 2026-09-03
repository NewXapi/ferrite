# `gateway-forward`

## 目录

```text
src/
├── lib.rs
├── adapter.rs
├── egress.rs
├── pipeline.rs
└── stream.rs
```

## 要实现

- 上游 URL、头和鉴权组装。
- 请求体转发。
- 流式与非流式响应处理。
- SSE usage 和 first-token 提取。
