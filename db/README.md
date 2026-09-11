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

## 当前启用状态（2026-09-11，开发环境 uf-local-postgres 容器）

- 容器镜像：`postgres:15-alpine` → `timescale/timescaledb:latest-pg15`（PG **15.18**，
  与原数据目录小版本严格对齐；`2.17.2-pg15` 带的是 15.10，小版本倒挂不要用）。
  同卷重建：named volume / 端口 5433 / 环境变量不变，容器内 17 个库全部无损。
- `shared_preload_libraries = 'timescaledb,pg_stat_statements'`（写在数据目录的
  `postgresql.auto.conf`）。**坑**：对 shared_preload_libraries 用 `ALTER SYSTEM
  SET ... = 'a,b'` 会把整串存成双引号的单个库名导致 PG 起不来（FATAL: could not
  access file "a,b"）——改 preload 列表时直接编辑 auto.conf 文件再重启容器。
- ferrite_smoke 库：`pg_stat_statements 1.10`、`pg_trgm 1.6`、`timescaledb 2.28.3`。
- `usage_logs` 已按 `db/optional/0004_timescale_usage_logs.sql` 转换：7 天 chunk、
  PK(id, created_at)、压缩策略 7 天。实测 51/53 chunk 压缩后 ~192KB（原 ~2.4MB，约 12x）。
- 转换前的保险 dump 在 `~/pg-backups/`（ferrite / ferrite_smoke / ferrite_e2e）。
- 其余 16 个库（其他项目）未创建扩展、未做任何变更；TS 预加载对未启用扩展的库
  只有约 MB 级内存开销。

## 演进摘要

| 文件 | 内容 |
|------|------|
| 0001 | 核心配置表（channels/groups/tokens/users）+ `group_name` → `groups TEXT[]` |
| 0002 | usage_logs（PK `(id, created_at)`，与 hypertable 对齐） |
| 0003 | model_prices |
| 0004 | api_models + monitor_history |
| 0005 | options + billing_redemptions + proxy_nodes |
| 0006 | `DROP TABLE route_units`（笛卡尔积物化表退役，展开改内存） |
