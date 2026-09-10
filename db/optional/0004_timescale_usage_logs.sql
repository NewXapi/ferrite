-- =====================================================================
-- 可选脚本：usage_logs 转 TimescaleDB hypertable（手动执行，不进 sqlx 迁移）
-- ---------------------------------------------------------------------
-- 前提：
--   1. PG 容器镜像为 timescale/timescaledb:<ver>-pg15（ferrite 开发环境已换，
--      详见 db/README.md「当前启用状态」）。
--   2. shared_preload_libraries 含 timescaledb。
--   3. 在目标库执行 CREATE EXTENSION IF NOT EXISTS timescaledb;（需超级用户）。
-- 幂等性：重复执行会在已存在的约束/策略上报错——脚本按"一次性转换"设计，
--   重复运行前先检查 timescaledb_information.hypertables。
-- 已知影响：
--   - PK 从 (id) 变为 (id, created_at)：hypertable 的唯一约束必须包含分区列；
--     应用侧按 id 查单行时若不带 created_at 仍可走 (id, created_at) 前缀索引。
--   - 压缩为 TSL 许可特性：自托管免费，禁止作为托管服务再分发。
-- =====================================================================

-- 1) 主键改造：唯一约束必须包含时间分区列
ALTER TABLE usage_logs DROP CONSTRAINT usage_logs_pkey;
ALTER TABLE usage_logs ADD PRIMARY KEY (id, created_at);

-- 2) 转 hypertable：7 天一个 chunk，migrate_data 把存量行迁入 chunk
SELECT create_hypertable(
    'usage_logs',
    'created_at',
    migrate_data       => true,
    chunk_time_interval => INTERVAL '7 days'
);

-- 3) 列压缩：按渠道/token 分段、时间倒序排布（对齐查询模式）
--    2026-09 实测：53 chunk 压缩 51 个，单 chunk ~2.4MB → ~192KB（约 12x）。
ALTER TABLE usage_logs SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'channel_key,token_key',
    timescaledb.compress_orderby   = 'created_at DESC, id DESC'
);

-- 4) 自动压缩策略：chunk 满 7 天后自动压缩
SELECT add_compression_policy('usage_logs', INTERVAL '7 days');

-- 5)（可选）对存量旧 chunk 立即压缩，不等策略触发：
-- SELECT compress_chunk(c) FROM show_chunks('usage_logs', older_than => INTERVAL '7 days') c;

-- 验证：
-- SELECT * FROM timescaledb_information.hypertables;
-- SELECT chunk_name, is_compressed FROM timescaledb_information.chunks ORDER BY range_start;
-- SELECT count(*) FROM usage_logs;   -- 行数应与转换前一致
