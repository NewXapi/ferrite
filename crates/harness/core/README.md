# `harness-core`

## 目录

```text
src/lib.rs
```

## 要实现

- Run、Step 和运行状态。
- 模型消息、工具调用、工具结果步骤。
- 最大步数、最大 token 和中止终止条件。
- AbortSignal。
- Run 序列化和恢复。

## 参考实现

| 能力 | 上游位置 | 机制 |
|------|---------|------|
| 步骤事件枚举 | `~/projects/harness/jcode/crates/jcode-message-types/src/lib.rs` `StreamEvent` | TextDelta / ToolUseStart / ToolInputDelta / ToolUseEnd / ToolResult / Thinking / TokenUsage |
| 运行事件枚举 | `~/projects/harness/zerostack/src/event.rs` `AgentEvent` | Token / Reasoning / ToolCall / ToolResult / Done / Error |
| 中止信号 | `~/projects/harness/jcode/crates/jcode-agent-runtime/src/lib.rs` `InterruptSignal` | `AtomicBool` + `Notify` + epoch 计数，`reset_if_epoch` 避免竞态 |
| 终止原因 | `~/projects/harness/pi-from-scratch/src/agent.ts` | end_turn / max_tokens / aborted / error 四个显式收尾状态 |
| 会话记录 | `~/projects/harness/oh-my-pi/packages/agent/src/compaction/entries.ts` `SessionEntry` | 带 `parentId` 的 id 树，可重放 |
| 工具调用配对 | `~/projects/harness/zerostack/src/session/mod.rs` `add_tool_call` / `add_tool_result` | 用 call id 关联调用与结果，不靠数组下标 |
