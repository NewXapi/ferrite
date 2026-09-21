# prompt

系统提示、角色资料、历史、变量展开与上下文裁剪。

## 职责

把 Agent 的角色资料、世界书、历史消息与运行时变量渲染成最终送模型的 prompt，并在
超出上下文预算时裁剪。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/render.rs` | prompt 渲染主流程 |
| `src/variables.rs` | 变量展开 |
| `src/world_info.rs` | 世界书条目注入 |
| `src/truncate.rs` | 上下文裁剪 |
| `src/reasoning.rs` | 推理内容处理 |
| `src/prompt_snapshot.rs` | prompt 快照（可复现/可审计） |
| `src/types.rs` | 公共类型 |
| `src/lib.rs` | crate 导出面 |

## 依赖

`core`。

## 硬约束

必须过 `wasm32` check——`tavern-state`（前端）直接依赖本 crate，是 web → harness 的
唯一越界边。

## 验收

```bash
cargo check -p harness-prompt
cargo check --target wasm32-unknown-unknown -p harness-prompt
cargo test -p harness-prompt               # CI
```
