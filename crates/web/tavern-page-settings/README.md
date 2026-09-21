# tavern-page-settings

连接与采样设置。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/lib.rs` | 设置视图 |

## 硬约束

必须过 `wasm32` check。

## 验收

```bash
cargo check -p tavern-page-settings
cargo check --target wasm32-unknown-unknown -p tavern-page-settings
```
