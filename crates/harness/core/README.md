# `harness-core`

## src/lib.rs

- `Run` — 一次 Agent 运行的 id、状态和步骤。
- `Step` — 模型文本、reasoning、工具调用、工具结果、错误和完成事件。
- `AbortSignal` — 运行中止状态。
- `RunSnapshot` — Run 序列化和恢复。

## 参考实现

- `/home/hathaway/projects/harness/jcode/crates/jcode-message-types/src/lib.rs` — StreamEvent。
- `/home/hathaway/projects/harness/jcode/crates/jcode-agent-runtime/src/lib.rs` — InterruptSignal。

## GenerationType

`AgentRun.generation_type` 使用 `GenerationType` 枚举（`chat` / `quiet` / `impersonate` / `continue`，serde snake_case）。非 `chat` 类型在 runtime 侧不注册工具；`continue` 输出拼接末条 assistant 文本（详见 `run.rs` 与 harness-runtime）。
