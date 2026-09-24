# forward

上游转发：URL/头/请求体/响应体/SSE 流的透传与协议适配。

## 职责

把 pipeline 上下文里的请求发出去，把上游响应（含流式 SSE）转回客户端。出口 Client
由 `egress` / `adapter_egress` 决定（直连或走 `proxy` 租借的节点）；`stream` 负责
SSE 分帧；`stream_resilience` 处理流中断。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/stage.rs` | 作为 Stage 挂进 pipeline；固化 `Timeouts` 与 retry 注入点 |
| `src/egress.rs` | `Timeouts`（connect / first_byte / total）与直接出口 |
| `src/adapter.rs` | 出口适配 trait |
| `src/adapter_egress.rs` | 把 `proxy` 的 Client 租借包成 egress |
| `src/stream.rs` | SSE 分帧与逐 chunk 下发 |
| `src/stream_resilience.rs` | 流中断恢复 |
| `src/pipeline.rs` | 转发子流程编排 |
| `src/lib.rs` | crate 导出面 |

## 关键语义

- `Timeouts::default()` = connect 5s / first_byte 30s / total 300s。
  `first_byte 30s` 意味着假死上游要干等 30s 才 failover——调度「慢」的主要来源之一。
- `apps/api` 用 `ForwardStage::new(egress, adaptors)`，`Timeouts` 与 retry policy 都是
  硬编码默认；`[gateway]` 配置段的 `gateway.timeout.first_byte_ms` /
  `gateway.retry.max_attempts` 选项已注册但**无消费方**。

## 依赖

`{pipeline, dispatch, protocol-bridge, proxy, metering}` + `contract`。

## 验收

```bash
cargo check -p forward
cargo test -p forward                      # CI（含 retry_wiring / adaptor_wiring 故障注入）
```
