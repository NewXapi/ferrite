# runtime

模型-工具循环、审批、持久化与步骤事件流。后端专用。

## 职责

驱动完整的 agentic loop：送 prompt → 收模型响应 → 若有 tool_call 则过审批门执行 →
把结果回喂模型，直到终止。期间发事件流、按需持久化。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/loop_engine.rs` | 循环引擎 |
| `src/turn.rs` | 单轮编排 |
| `src/tool_exec.rs` | 工具执行 |
| `src/persistence.rs` | 持久化 |
| `src/provider.rs` | 模型 provider 抽象 |
| `src/delegation.rs` | 子任务委派 |
| `src/delta_agg.rs` | 流式增量聚合 |
| `src/cancel.rs` | 取消传播 |
| `src/event_sink.rs` | 事件下沉 |
| `src/bias.rs` | token 偏置（encode 由调用方注入，无 tokenizer 依赖） |
| `src/lib.rs` | crate 导出面 |

## 依赖

`{core, prompt, tools}`。

## 验收

```bash
cargo check -p harness-runtime
cargo test -p harness-runtime              # CI
```
