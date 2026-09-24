# tavern-page-lorebook

世界书管理。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/lib.rs` | 世界书管理视图 |

## 硬约束

必须过 `wasm32` check。面板被 `tavern-page-characters` 内嵌复用。

## 验收

```bash
cargo check -p tavern-page-lorebook
cargo check --target wasm32-unknown-unknown -p tavern-page-lorebook
```
