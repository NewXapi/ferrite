# `harness-runtime`

Backend-only Agent loop. UI belongs to `tavern-web`. HTTP APIs, login, group chat, macros and provider protocol adapters are out of scope.

## src/

- `cancel.rs` — cloneable `CancellationToken` on `tokio::sync::watch`.
- `provider.rs` — injected `ChatProvider` plus normalized request/delta types（含 `response_format` OpenAI 透传）。
- `delta_agg.rs` — text/reasoning/tool-call fragment aggregation.
- `event_sink.rs` — `AgentRunEvent` factory, vec sink, mpsc sink.
- `turn.rs` — one streaming model turn.
- `tool_exec.rs` — `ToolRequestGate`, object-schema subset, handler dispatch.
- `persistence.rs` — `run.json`, `events.jsonl`, tool I/O, checkpoints.
- `delegation.rs` — 委派/子 run 驱动：gate 校验（深度/数量/开关）、invocations+tasks 落盘（append-log）、结果预算截断回灌。
- `loop_engine.rs` — bounded model → tool → model loop, model retry, run-state loading;stealth 工具（`annotations.stealth`）纯轮直接终结 run，混合轮回灌并在事件带 stealth 标记。
  `AgentRunRequest.generation_type`（`GenerationType`）：非 `chat` 类型不注册工具
  （对齐 ST `canPerformToolCalls` 的 `noToolCallTypes`）；`continue` 类型把末条
  assistant 文本作为前缀拼进输出。

## Verify

```sh
cargo test -p harness-runtime
cargo check -p harness-runtime
```
