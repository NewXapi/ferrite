-- =====================================================================
-- 0002 用量流水：usage_logs
-- ---------------------------------------------------------------------
-- 工作负载画像：
--   写 —— 每请求一条 INSERT，全库唯一的热写路径。逐条异步写起步，
--         量上来后改内存 buffer 批量 flush（接口已留）。
--   读 —— 管理台按时间倒序分页 / 按用户查账单；统计分析走未来聚合表。
-- 权威账本语义：api_tokens.used_quota / auth_users.used_quota 是本表的
--   缓存态；进程重启后可由本表重算——本表有 WAL，计费数据不允许丢。
-- [演进] +group_id / +status_code / +upstream_request_id（2026-09）：
--   group_id 用于倍率追溯；status_code + log_type=5 让失败请求进入
--   统计口径（此前非 2xx 完全不记录）；upstream_request_id 供上游对账。
-- [索引演进] 删除 idx_usage_logs_token_name / idx_usage_logs_model_name：
--   每个索引都是写放大，token/model 维度统计属低频分析查询，
--   未来由聚合表承担，不在热写表上养索引。
-- 调优备忘：月增 10 万行以上时转 TimescaleDB hypertable（见 db/optional/）。
-- [演进 v2] PK 直接定为 (id, created_at)：与 hypertable 的唯一约束要求对齐，
--   TS 转换无需再改主键。代价：裸 PG 表上不再由约束保证 id 单列唯一
--   （id 来自 BIGSERIAL 序列，实际恒唯一）；按 id 单列查询仍走 PK 前缀索引。
-- =====================================================================
CREATE TABLE IF NOT EXISTS usage_logs (
    id                  BIGSERIAL,
    log_type            SMALLINT NOT NULL DEFAULT 2,  -- 1=充值 2=消费 5=错误（对齐 new-api 枚举）
    user_key            UUID NOT NULL,                -- → auth_users.key
    username            TEXT NOT NULL DEFAULT '',     -- 用户名冗余（用户改名不改历史日志）
    token_key           UUID,                         -- → api_tokens.key；系统调用可空
    token_name          TEXT NOT NULL DEFAULT '',     -- token 名冗余
    channel_key         UUID,                         -- → api_channels.key；本次实际命中的渠道
    channel_name        TEXT NOT NULL DEFAULT '',     -- 渠道名冗余
    model_name          TEXT NOT NULL DEFAULT '',     -- 公开别名（客户端请求的模型名）
    prompt_tokens       INT NOT NULL DEFAULT 0,       -- 输入 tokens（上游 usage；缺失时为估算值）
    completion_tokens   INT NOT NULL DEFAULT 0,       -- 输出 tokens（同上）
    quota               BIGINT NOT NULL DEFAULT 0,    -- 本次实扣（内部单位 500_000=$1，已含分组倍率）
    use_time_ms         INT NOT NULL DEFAULT 0,       -- 全链路耗时（网关侧计时）
    is_stream           BOOLEAN NOT NULL DEFAULT false,
    group_id            TEXT NOT NULL DEFAULT '',     -- 生效分组（token 优先，回落 user）；倍率追溯键
    status_code         SMALLINT NOT NULL DEFAULT 0,  -- 上游响应码；0=非上游错误（网关自身拒绝）
    upstream_request_id TEXT NOT NULL DEFAULT '',     -- 上游返回的 request id；排障对账
    ip                  TEXT NOT NULL DEFAULT '',     -- 客户端 IP
    request_id          TEXT NOT NULL DEFAULT '',     -- 网关侧请求 id
    content             TEXT NOT NULL DEFAULT '',     -- 扩展信息（错误摘要等；JSON 字符串）
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE usage_logs ADD COLUMN IF NOT EXISTS group_id            TEXT NOT NULL DEFAULT '';
ALTER TABLE usage_logs ADD COLUMN IF NOT EXISTS status_code         SMALLINT NOT NULL DEFAULT 0;
ALTER TABLE usage_logs ADD COLUMN IF NOT EXISTS upstream_request_id TEXT NOT NULL DEFAULT '';

-- 热写表只养两个索引（写放大控制）；被删的两个低频分析索引见头部[索引演进]
CREATE INDEX IF NOT EXISTS idx_usage_logs_created ON usage_logs(created_at);        -- 时间倒序分页（管理台默认视图）
CREATE INDEX IF NOT EXISTS idx_usage_logs_user    ON usage_logs(user_key, created_at); -- "某用户的账单"页
DROP INDEX IF EXISTS idx_usage_logs_token_name;  -- [演进] 统计走聚合表，热表减负
DROP INDEX IF EXISTS idx_usage_logs_model_name;  -- [演进] 同上

-- [演进 v2] 主键统一收敛为 (id, created_at)：
--   旧库（PK=id 单列）在此处摘掉重建；新库建表时未定义主键，此处一次成型。
ALTER TABLE usage_logs DROP CONSTRAINT IF EXISTS usage_logs_pkey;
ALTER TABLE usage_logs ADD CONSTRAINT usage_logs_pkey PRIMARY KEY (id, created_at);
