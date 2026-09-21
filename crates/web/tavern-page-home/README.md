# tavern-page-home

品牌落地页。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/lib.rs` | 落地页视图 |

## 硬约束

必须过 `wasm32` check。

## 验收

```bash
cargo check -p tavern-page-home
cargo check --target wasm32-unknown-unknown -p tavern-page-home
```
