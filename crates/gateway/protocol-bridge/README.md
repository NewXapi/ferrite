# `gateway-protocol-bridge`

数据面协议适配层 —— 厂商协议兼容的集中地。对标 new-api `relay/channel/*/adaptor.go`。

## 架构：单格式双向 codec + 恒两跳

每个协议实现**一个** `FormatCodec`（本格式 ↔ IR 双向），格式互转恒定两跳：

```text
客户端格式体 ──decode──▶ IR ──encode──▶ 上游格式体      （请求方向）
上游格式体   ──decode──▶ IR ──encode──▶ 客户端格式体    （响应方向）
```

N 种格式只需 N 个 codec，而不是 N×(N-1) 个有向转换器；**加一个格式只写一个文件**。

流式响应由每条流一个的 `StreamEncoder` 编码（`&mut self` 持有 block 边界与 tool
碎片累积状态）；解码侧 `decode_event` 保持无状态——Claude 的 `input_json_delta`
到 OpenAI 的 `tool_calls[].arguments` 是逐事件 1:1 映射，真正需要跨事件状态的是
编码侧（Claude/Responses 要补 `content_block_start` / `output_item.added` 边界帧）。

## 文件

- `src/ir.rs` — provider-neutral 中间表示（`LlmRequest` / `LlmResponse` /
  `StreamEvent` / `ContentBlock` / `Usage`），带未知字段逃逸口（`extra` /
  `Unknown{raw}`），保证跨格式 round-trip 不丢信息。
- `src/format_codec.rs` — `FormatCodec` / `StreamEncoder` trait + `FormatRegistry`
  （单格式注册表 + `translate_request` / `translate_response` 两跳封装；
  同格式或任一侧 `Passthrough` 时零转换直通）。
- `src/openai.rs` — OpenAI Chat Completions codec。
- `src/claude.rs` — Anthropic Messages codec（含**入站解码** + 状态流 tool 装配）。
- `src/gemini.rs` — Gemini `generateContent` codec（含 finishReason 映射表）。
- `src/responses.rs` — OpenAI Responses codec（含 SSE 事件生命周期补齐）。
- `src/adaptor.rs` — 共享词汇：`Protocol`（格式标识）+ `AdaptorError`。
- `src/sse.rs` — SSE 帧扫描（逐字保真透传 + 事件检测 + 供转换消费的完整帧）。
- `src/error_mapping.rs` — `contract::error::NormalizedError` → Chat / Responses /
  Anthropic / Gemini 四种错误形状。
- `src/stage.rs` — pipeline Stage 4：错误映射 + 把 forward 备好的响应打包成 HTTP 响应。

## 与相关 crate 的边界

| crate | 角色 |
|-------|------|
| `gateway-forward` | IO 编排 + 双向转换闭环（请求与响应方向必须协作同一份 codec） |
| `gateway-protocol-bridge` | 转换规则本身（纯函数，无 IO） |
| `contract::error::NormalizedError` | 跨 crate 单一错误协议 |

转换之所以在 forward 内闭环、而不在 stage 里：请求方向与响应方向必须用同一份
codec 协作，而入站格式（请求路径判定）与上游格式（渠道 `provider_type`）只有在
`forward_once` 里才同时可见。旧实现把响应转换放在本 crate 的 stage 里并硬编码
「响应源协议 = OpenAI」，流式路径更是完全不转换——流式的 Claude/Gemini 客户端
只能拿到 OpenAI 事件形状。

原 `protocol` crate 已删除，其 SseScanner / ProtocolError 迁入 `sse` / `adaptor`。

## 验收

```sh
cargo test -p gateway-protocol-bridge
cargo clippy -p gateway-protocol-bridge --all-targets
```
