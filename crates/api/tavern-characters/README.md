# tavern-characters

角色卡 CRUD，含 PNG 元数据处理。

## 职责

角色卡的增删改查。角色卡以 PNG 为载体，元数据写在 PNG 的 tEXt chunk 里，`png.rs`
负责 chunk 级读写（不重编码图片）。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/http.rs` | axum 端点 |
| `src/lib.rs` | 角色卡业务逻辑 |
| `src/png.rs` | PNG tEXt chunk 级读写 |

## 验收

```bash
cargo check -p tavern-characters
cargo test -p tavern-characters            # CI
```
