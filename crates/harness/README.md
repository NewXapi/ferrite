# harness — Agent 运行时

Agent 执行循环的 crate 群。依赖方向：`runtime → {core, prompt, tools}`、`prompt → core`、
`tools → core`。

## crate 清单

| crate | 职责 | 关键模块 |
|---|---|---|
| `core` | Run/Step 状态机、取消与序列化（零 runtime 依赖） | `run` / `status` / `event` / `plan` / `profile` / `storage` / `workspace_path` |
| `prompt` | 系统提示、角色资料、历史、变量展开与上下文裁剪 | `render` / `truncate` / `variables` / `world_info` / `reasoning` / `prompt_snapshot` / `types` |
| `tools` | 工具契约（ToolSpec/ToolCall/ToolResult），只声明不执行 | `spec` / `format` / `gate` / `result` / `adapter` |
| `runtime` | 模型-工具循环、审批、持久化、步骤事件流（仅后端） | `loop_engine` / `turn` / `tool_exec` / `persistence` / `provider` / `delegation` / `delta_agg` / `cancel` / `event_sink` / `bias` |
| `tokenizer` | 真实 tokenizer | `engine` / `registry` |
| `vectors` | 向量检索记忆 | `chunk` / `hash` / `index` / `recall` |

## 硬约束

- `core` / `prompt` / `tools` 必须支持 `wasm32-unknown-unknown`：
  `cargo check --target wasm32-unknown-unknown -p <crate>`
- `runtime` / `tokenizer` / `vectors` 是后端专用，不做 wasm 约束
- `tokenizer` **零消费方**（`harness-runtime/src/bias.rs` 的 encode 由调用方注入）
- `vectors` **未接入 workspace members 且零消费方**——改它不会进任何构建

## 验收

```bash
cargo check -p harness-core -p harness-prompt -p harness-tools \
  -p harness-runtime -p harness-tokenizer -p harness-vectors
cargo check --target wasm32-unknown-unknown -p harness-core -p harness-prompt -p harness-tools
```
