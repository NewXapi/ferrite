# tavern-presets

单用户预设 JSON 文件读写。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/http.rs` | axum 端点 |
| `src/lib.rs` | 预设存取 |

## 验收

```bash
cargo check -p tavern-presets
cargo test -p tavern-presets               # CI
```
