-- =====================================================================
-- 0004 模型目录 + 渠道探活历史
-- ---------------------------------------------------------------------
-- api_models  —— 对外售卖的模型条目（admin-data 域管理）；读多写少。
-- monitor_history —— 渠道探活结果流水（每探活一条 INSERT，append-only）；
--   与 usage_logs 同属"时间有序流水"，量大时同样可转 hypertable
--   （PK 需含 created_at，转换前先改主键，见 db/optional/0004 的注意事项）。
-- =====================================================================

-- api_models — 模型目录（读：管理台列表/快照；写：管理台 CRUD；行数十级）
CREATE TABLE IF NOT EXISTS api_models (
    key           UUID PRIMARY KEY,               -- 模型条目 id
    name          TEXT UNIQUE NOT NULL,           -- 模型名（展示/唯一键）
    owner         TEXT NOT NULL DEFAULT '',       -- 归属方（维护者标记）
    model_type    TEXT NOT NULL DEFAULT 'chat',   -- 模型类别（chat/embedding/...）
    base_url      TEXT NOT NULL DEFAULT '',       -- 独立上游地址（可空串=跟随渠道）
    api_key       TEXT NOT NULL DEFAULT '',       -- 独立凭据（可空串=跟随渠道；明文，同 api_channels.keys 现状）
    capabilities  JSONB NOT NULL DEFAULT '[]',    -- 能力标签数组（vision/tool/...）
    speed         INT NOT NULL DEFAULT 0,         -- 主观速度分（展示用）
    rating        JSONB NOT NULL DEFAULT '{}',    -- 评分对象（展示用）
    usage_count   BIGINT NOT NULL DEFAULT 0,      -- 累计调用次数（展示态）
    max_tokens    INT NOT NULL DEFAULT 0,         -- 最大上下文 tokens；0=未设
    is_vision     BOOLEAN NOT NULL DEFAULT false, -- 支持图像输入
    is_tool       BOOLEAN NOT NULL DEFAULT false, -- 支持工具调用
    status        SMALLINT NOT NULL DEFAULT 1,    -- 1=上架（其余=下架）
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_api_models_name       ON api_models(name);
CREATE INDEX IF NOT EXISTS idx_api_models_owner      ON api_models(owner);
CREATE INDEX IF NOT EXISTS idx_api_models_model_type ON api_models(model_type);
CREATE INDEX IF NOT EXISTS idx_api_models_status     ON api_models(status);
CREATE INDEX IF NOT EXISTS idx_api_models_created_at ON api_models(created_at DESC);

-- monitor_history — 渠道探活历史（写多读少：每探活一条；调优点：月增 10 万行
--   以上转 hypertable；channel_key 维度查询已有复合索引兜底）
CREATE TABLE IF NOT EXISTS monitor_history (
    id            BIGSERIAL PRIMARY KEY,
    channel_key   UUID NOT NULL,                 -- → api_channels.key
    channel_name  TEXT NOT NULL DEFAULT '',      -- 渠道名冗余
    model         TEXT NOT NULL DEFAULT '',      -- 探活用模型名
    ok            BOOLEAN NOT NULL,              -- 探活是否成功
    status_code   INT,                           -- 上游响应码；传输层失败为 NULL
    latency_ms    INT NOT NULL DEFAULT 0,        -- 往返耗时
    error_kind    TEXT NOT NULL DEFAULT '',      -- 错误分类（timeout/connect/...）
    message       TEXT NOT NULL DEFAULT '',      -- 错误摘要
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_monitor_history_channel
    ON monitor_history(channel_key, created_at);
