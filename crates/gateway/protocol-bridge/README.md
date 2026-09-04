# `gateway-protocol-bridge`

数据面协议适配层 — 厂商协议兼容的集中地。对标 new-api `relay/channel/*/adaptor.go`
（每个厂商一个适配器，负责客户端协议 ↔ 厂商协议的双向转换）。

## 文件

- `src/adaptor.rs` — 厂商适配器：`Protocol` 枚举、`Codec` trait（请求/响应字节
  转换）、`AdaptorRegistry`（`(source, target)` → Codec 查找）+ 内置适配器
  （OpenAi 透传 / Claude / Gemini 转换骨架）。
- `src/sse.rs` — SSE 帧扫描（事件边界 / keepalive / 终止），源自原 protocol crate。
- `src/error_mapping.rs` — `contract::error::NormalizedError` → OpenAI /
  Anthropic / Gemini 各自规范的错误响应体。
- `src/stage.rs` — pipeline Stage 4：把上游响应经适配器转为客户端协议。

## 与相关 crate 的边界

| crate | 角色 |
|-------|------|
| `gateway-forward` | IO 编排（发上游 / 流回传），不碰协议转换 |
| `gateway-protocol-bridge` | 协议转换（纯函数，无 IO） |
| `contract::error::NormalizedError` | 跨 crate 单一错误协议 |

原 `protocol` crate 已删除，其 Codec trait / SseScanner / ProtocolError 迁入本
crate 的 `adaptor` / `sse` 模块。

## 厂商适配器（TODO）

- `OpenAiCodec` — 中枢格式透传（TODO #510/#511）
- `ClaudeCodec` — OpenAI↔Claude 双向（TODO #512/#513）
- `GeminiCodec` — OpenAI↔Gemini 双向（TODO #514/#515）

## 验收

```sh
cargo test -p gateway-protocol-bridge
cargo clippy -p gateway-protocol-bridge --all-targets
```