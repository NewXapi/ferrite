# vectors

向量检索记忆：chunk、哈希索引与召回。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/chunk.rs` | 文本分块 |
| `src/hash.rs` | 局部敏感哈希 |
| `src/index.rs` | 向量索引 |
| `src/recall.rs` | 召回 |
| `src/lib.rs` | crate 导出面 |

## 当前状态

**未接入 workspace members 且零消费方**——本 crate 不在根 `Cargo.toml` 的 members 里，
默认构建/CI 不会编译它。改它不会影响任何生产链路；接入前先改 `workspace.members`。

## 验收

```bash
cargo check -p harness-vectors
cargo test -p harness-vectors              # CI（需先在 workspace.members 声明）
```
