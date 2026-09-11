# db-bootstrap

sqlx 迁移的单一权威入口。生产装配（`admin-router::router`）与集成测试的建表一律走本 crate：`db/migrations/` 是唯一的 schema 事实源。

## 文件

- `src/lib.rs` — `run_migrations(pool)`：按文件名版本号顺序在事务中执行未应用的迁移；失败返回 `MigrateError`，调用方决定日志/退出策略。

## 约定

- 迁移文件全部幂等（`IF NOT EXISTS` / `ON CONFLICT` / `DROP IF EXISTS`），对存量库与全新库都能安全执行。
- 已应用版本记录在 `_sqlx_migrations` 表。

## 验收

```sh
cargo check -p db-bootstrap --all-targets
```
