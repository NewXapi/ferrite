# tavern-settings

用户设置读存。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/http.rs` | axum 端点 |
| `src/lib.rs` | 设置读存 |

## 验收

```bash
cargo check -p tavern-settings
cargo test -p tavern-settings              # CI
```
