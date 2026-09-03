# `tavern-generate`

## 目录

```text
src/lib.rs
```

## 要实现

- OpenAI 兼容生成请求转发。
- 请求头密钥注入。
- SSE 流透传。
- 客户端中止传播到上游请求。
- 模型列表和连通状态查询。

## 参考实现

| 能力 | 上游位置 | 机制 |
|------|---------|------|
| SSE 透传 | `~/projects/SillyTavern/src/util.js:709` `forwardFetchResponse` | 上游响应头与流体逐帧转发给客户端 |
| 中止 | `~/projects/SillyTavern/src/endpoints/backends/chat-completions.js:224` | `AbortController` 绑到 `request.socket` 的 close 事件 |
| 流扫描与取消源 | `~/projects/new-api/apps/api/relay/helper/stream_scanner.go:77` `StreamScannerHandler` | 扫描 + dataChan + ping goroutine，多个取消源汇聚到 stopChan |
| 流中错误抑制 | `~/projects/new-api/apps/api/controller/relay.go:78` `canWriteErrorBody` | 已开始写流后不再插 JSON 错误体 |
| 事件流接口 | `~/projects/new-api/apps/api/internal/transport/contract/stream.go:12` `EventStream` | SetHeaders / WriteNamedEvent / Flush / Done |
