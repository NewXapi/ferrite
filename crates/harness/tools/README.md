# tools

工具契约：只声明工具形状，不执行。

## 职责

定义 `ToolSpec`（工具声明）/ `ToolCall`（模型发起的调用）/ `ToolResult`（回给模型的
结果）三类契约，以及审批门（`gate`）与结果格式化。执行在 `runtime` 的 `tool_exec` 里。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/spec.rs` | `ToolSpec` 工具声明 |
| `src/format.rs` | 参数/结果格式化 |
| `src/gate.rs` | 审批门 |
| `src/result.rs` | `ToolResult` |
| `src/adapter.rs` | 适配层 |
| `src/lib.rs` | crate 导出面 |

## 依赖

`core`。

## 硬约束

必须过 `wasm32` check。

## 验收

```bash
cargo check -p harness-tools
cargo check --target wasm32-unknown-unknown -p harness-tools
cargo test -p harness-tools                # CI
```
