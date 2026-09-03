# `gateway-protocol-bridge`

## 文件

- `src/lib.rs` — 公开 ProtocolBridgeStage 和 codec registry。
- `src/codec.rs` — 按客户端协议和渠道协议选择 codec。
- `src/stage.rs` — 把 forward 的上游响应转换成客户端协议响应。
- `src/error_mapping.rs` — 把 NormalizedError 转换成协议错误体。

