# db — SQL 迁移目录

## 约定

- 迁移文件用 **sqlx migrate** 格式：`<版本号>_<描述>.sql`（如 `0001_init_core_tables.sql`）。
  版本号单调递增，**永不修改已合入的旧文件**——schema 演进一律新增文件。
- 接线：`sqlx::migrate!("db/migrations")` 在进程启动时自动按版本应用，
  已应用记录存 `_sqlx_migrations` 表。部署即建表，可复刻。
- 应用侧各 crate 的 `ensure_table`（CREATE/ALTER IF NOT EXISTS 补丁）在迁移
  接线后退役；Rust 侧只保留"启动时跑迁移"一个调用点。
- 每个文件头部必须写**工作负载注释**（读/写比、预期行数量级、热列、调优点），
  每列后带 `--` 用途注释——这是给未来调优者的现场记录，不是装饰。
- 演进历史写在文件内的 `[演进 vX]` 注释块里：动因 + 日期，配合幂等的
  `ADD/DROP COLUMN IF EXISTS` + 回填语句，保证全新库与旧库两条路径都能跑通。

## 事务

sqlx 默认把每个迁移文件包在事务里执行；本目录禁止
`CREATE INDEX CONCURRENTLY` / `ALTER TYPE ... ADD VALUE` 等不能进事务的语句。

## 可选插件（未接线，按需启用）

- **TimescaleDB**：usage_logs 转 hypertable（时间分区 + 列压缩 + 连续聚合）。
  启用前提：PG 安装 timescaledb 扩展。转换脚本要点：
  `SELECT create_hypertable('usage_logs', 'created_at', migrate_data => true)`，
  且 **PK(id) 必须改为 PK(id, created_at)**——hypertable 的唯一约束必须包含分区列。
- **pg_stat_statements**：慢查询统计，建议常开（`shared_preload_libraries`）。
- 其余插件评估见 PR 讨论，不在基础迁移中引入任何扩展依赖（CI/开发库保持零门槛）。
