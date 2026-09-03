# `harness-runtime`

## src/lib.rs

- `run` — 模型调用、工具调用、结果回灌循环。
- `ToolExecutor` — 执行器注册和按工具名分发。
- `RunStore` — Run 追加保存和恢复。
- `StepEventStream` — 把步骤作为 SSE 发给前端。
- `Approval` — 需要用户确认的工具执行请求。

