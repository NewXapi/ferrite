# core

Agent Run/Step 状态机、取消与序列化。harness 域的零依赖底座。

## 职责

定义一个 Agent Run 从创建到终态的完整状态模型，以及可序列化的事件类型。只做数据与
纯逻辑，不依赖 tokio/sqlx/reqwest，因此可编 `wasm32`。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/run.rs` | Run 状态机与转换 |
| `src/status.rs` | 状态词表 |
| `src/event.rs` | 运行时事件类型（可序列化） |
| `src/plan.rs` | 执行计划 |
| `src/profile.rs` | Agent 角色资料 |
| `src/profile_diagnostic.rs` | 资料诊断 |
| `src/storage.rs` | 持久化 trait/形状 |
| `src/workspace_path.rs` | 工作区路径解析与越界防护 |
| `src/lib.rs` | crate 导出面 |

## 硬约束

必须过 `cargo check --target wasm32-unknown-unknown -p harness-core`。

## 验收

```bash
cargo check -p harness-core
cargo check --target wasm32-unknown-unknown -p harness-core
cargo test -p harness-core                 # CI
```
