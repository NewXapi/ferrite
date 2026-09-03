# `harness-runtime`

Backend-only Agent loop. UI belongs to `tavern-web`. HTTP APIs, login, group chat, macros and provider protocol adapters are out of scope.

## src/

- `cancel.rs` — cloneable `CancellationToken` on `tokio::sync::watch`.
- `provider.rs` — injected `ChatProvider` plus normalized request/delta types.
- `delta_agg.rs` — text/reasoning/tool-call fragment aggregation.
- `event_sink.rs` — `AgentRunEvent` factory, vec sink, mpsc sink.
- `turn.rs` — one streaming model turn.
- `tool_exec.rs` — `ToolRequestGate`, object-schema subset, handler dispatch.
- `persistence.rs` — `run.json`, `events.jsonl`, tool I/O, checkpoints.
- `loop_engine.rs` — bounded model → tool → model loop.

## Verify

```sh
cargo test -p harness-runtime
cargo check -p harness-runtime
```
