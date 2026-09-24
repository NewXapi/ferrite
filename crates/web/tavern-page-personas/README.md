# tavern-page-personas

用户人格管理。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/lib.rs` | 人格管理视图 |

## 硬约束

必须过 `wasm32` check。面板被 `tavern-page-characters` 内嵌复用。

## 验收

```bash
cargo check -p tavern-page-personas
cargo check --target wasm32-unknown-unknown -p tavern-page-personas
```
