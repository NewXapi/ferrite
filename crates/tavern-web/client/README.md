# `tavern-client`

## 目录

```text
src/lib.rs
```

## 要实现

- 角色、聊天、设置和密钥 API 客户端。
- 生成请求和 SSE 消费。
- 请求错误转换。

## 参考实现

| 能力 | 上游位置 | 机制 |
|------|---------|------|
| SSE 消费 | `~/projects/SillyTavern/public/scripts/sse-stream.js:10` | 自建 SSE 分帧，不用 EventSource（要 POST） |
