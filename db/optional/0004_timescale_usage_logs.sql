-- =====================================================================
-- 可选脚本：usage_logs 转 TimescaleDB hypertable（手动执行，不进 sqlx 迁移）
-- ---------------------------------------------------------------------
-- 前提：
--   1. PG 容器镜像为 timescale/timescaledb:<ver>-pg15（ferrite 开发环境已换，
--      详见 db/README.md「当前启用状态」）。
--   2. shared_preload_libraries 含 timescaledb。
--   3. 在目标库执行 CREATE EXTENSION IF NOT EXISTS timescaledb;（需超级用户）。
-- 幂等性：整体幂等——已是 hypertable 则跳过转换，已有压缩策略则不重复添加，
--   可安全重跑（ocr 审查修复：原版缺 guard，重跑会在约束/策略上报错）。
-- 锁表提示：migrate_data => true 会在转换期间锁表并复制存量数据——45k 行秒级
--   完成；百万行以上请在维护窗口执行（ocr 审查建议）。
-- 已知影响：
--   - 主键无需改动：基线迁移（0002）已将 PK 定为 (id, created_at)，
--     满足 hypertable "唯一约束必须包含分区列" 的要求。
--   - 压缩为 TSL 许可特性：自托管免费，禁止作为托管服务再分发。
-- =====================================================================

BEGIN;

-- 1) 转 hypertable（幂等：已转换则跳过），7 天一个 chunk
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM timescaledb_information.hypertables
        WHERE hypertable_schema = 'public' AND hypertable_name = 'usage_logs'
    ) THEN
        PERFORM create_hypertable(
            'usage_logs',
            'created_at',
            migrate_data        => true,
            chunk_time_interval => INTERVAL '7 days'
        );
    ELSE
        RAISE NOTICE 'usage_logs 已是 hypertable，跳过 create_hypertable';
    END IF;
END $$;

-- 2) 列压缩：按渠道/token 分段、时间倒序排布（对齐查询模式）
--    2026-09 实测：53 chunk 压缩 51 个，单 chunk ~2.4MB → ~192KB（约 12x）。
--    SET 关系选项本身幂等，重复执行无副作用。
ALTER TABLE usage_logs SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'channel_key,token_key',
    timescaledb.compress_orderby   = 'created_at DESC, id DESC'
);

-- 3) 自动压缩策略（幂等：已有策略则不重复添加）
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM timescaledb_information.jobs
        WHERE proc_name = 'policy_compression' AND hypertable_name = 'usage_logs'
    ) THEN
        PERFORM add_compression_policy('usage_logs', INTERVAL '7 days');
    ELSE
        RAISE NOTICE 'usage_logs 已有压缩策略，跳过 add_compression_policy';
    END IF;
END $$;

COMMIT;

-- 4)（可选）对存量旧 chunk 立即压缩，不等策略触发：
-- SELECT compress_chunk(c) FROM show_chunks('usage_logs', older_than => INTERVAL '7 days') c;

-- 验证：
-- SELECT * FROM timescaledb_information.hypertables;
-- SELECT chunk_name, is_compressed FROM timescaledb_information.chunks ORDER BY range_start;
-- SELECT count(*) FROM usage_logs;   -- 行数应与转换前一致
