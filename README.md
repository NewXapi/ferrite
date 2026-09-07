# proto: shoes 客户端协议链基础设施（骨架）

从 shoes（MIT，https://github.com/cfal/shoes）移植。零额外进程的多协议出口。

本 PR 只搬基础设施；协议握手实现见后续 PR。

## 模块

- `async_stream.rs` — `AsyncStream` / `AsyncMessageStream` 流抽象
- `address.rs` — `Address` / `NetLocation` / `ResolvedLocation`
- `stream_reader.rs` — 握手期读缓冲
- `proxy_connector.rs` — `ProxyConnector` trait

## 验收

```bash
cargo test -p gateway-proxy
```
