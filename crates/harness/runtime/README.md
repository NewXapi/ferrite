# `harness-runtime`

## 目录

```text
src/lib.rs
```

## 要实现

- 模型调用、工具调用、结果回灌循环。
- 工具执行器注册和分发。
- 通过 gateway 调用模型。
- Run 持久化和恢复。
- SSE 步骤事件流。
- 工具超时、审批和执行环境。

## 参考实现

| 能力 | 上游位置 | 机制 |
|------|---------|------|
| 工具执行 | `~/projects/SillyTavern/public/scripts/tool-calling.js:325` `invokeFunctionTool` | 按名字分发到实现，结果回灌下一轮 |
| 取消传播 | `~/projects/new-api/apps/api/relay/helper/stream_scanner.go:77` | 多取消源汇聚 stopChan，`wg.Wait` 收尾 |
| 最小循环 | `~/projects/harness/pi-from-scratch/src/agent.ts` | stream → push assistant → 执行 tool_calls → push tool_results |
| 手写循环 | `~/projects/harness/zerostack/src/agent/runner.rs` `spawn_agent` | 不用框架的 max_turns，自己控制续跑，便于插 hook |
| 步数与超时 | `~/projects/harness/oh-my-pi/packages/agent/src/agent-loop.ts` `runLoopBody` | stepCounter + `AbortSignal.any` 合并 deadline |
| 并行工具与中止收尾 | `~/projects/harness/deepseek-harness/packages/core/agent-loop/src/tool-calls.ts` `executeToolCalls` | 有界滚动池；中止时排空已启动的，未启动的补合成结果 |
| 工具上下文 | `~/projects/harness/jcode/crates/jcode-tool-core/src/lib.rs` `ToolContext` | session_id / working_dir / 关闭信号 / 执行模式随调用传入 |
| 权限判定 | `~/projects/harness/zerostack/src/permission/checker.rs` `CheckResult` | Allowed / AllowedWithCoaching / Ask / Denied，含死循环检测 |
| 执行隔离 | `~/projects/harness/zerostack/src/sandbox.rs` `Sandbox` | bwrap 后端、network_unshare、timeout、kill_active |
| 追加式持久化 | `~/projects/harness/oh-my-pi/packages/coding-agent/src/session/indexed-session-storage.ts` | 按路径串行化的有序追加队列 + 本地索引 |
| 暂停闸 | `~/projects/harness/oh-my-pi/packages/agent/src/pause.ts` `AgentPauseGate` | 在回合边界检查，abort 时释放 |
