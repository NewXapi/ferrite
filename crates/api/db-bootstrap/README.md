# db-bootstrap

sqlx 迁移的单一权威入口。

## 职责

启动时按序应用 `db/migrations/*.sql`。**所有** schema 变更必须走这里，禁止其他
crate 自己建表（`auth` 的 `ddl` 历史遗留除外，已标记）。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/lib.rs` | 迁移执行入口 |

## 验收

```bash
cargo check -p db-bootstrap
```
