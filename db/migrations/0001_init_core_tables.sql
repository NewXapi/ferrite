-- =====================================================================
-- 0001 核心配置表：渠道 / 分组 / 令牌 / 账号 / 会话
-- ---------------------------------------------------------------------
-- 工作负载画像：
--   读 —— 启动与 reload 时全量扫描构建内存快照（行数：渠道几十、分组个位、
--         token 百~千、用户百级），热路径永远不查 PG（数据面只读内存快照）。
--   写 —— 管理台 CRUD（低频）；唯一热写点是 api_tokens.used_quota，
--         设计为「内存结算 + 周期批量 flush」，权威账本在 usage_logs。
-- 幂等性：本文件对「全新库」与「已被旧版 ensure_table 建过表的库」都能执行：
--   全新库 = CREATE 后立即走文件内的形态演进（见各 [演进] 注释）；
--   旧库   = CREATE IF NOT EXISTS 跳过，ALTER 补列 + 回填 + 删旧列。
-- 调优备忘：配置表不建外键（刻意的）——读侧是快照全量扫描，写入顺序由
--   管理台保证，省掉外键锁开销。行数量级不变大，无需分区。
-- =====================================================================

-- ---------------------------------------------------------------------
-- api_channels — 渠道事实源（路由的唯一数据源；派生索引在内存快照里展开）
-- [演进] v2 (2026-09)：group_name TEXT 单值 → groups TEXT[] 多值。
--   动因：一个渠道服务多个分组（vip/default 共享上游），单值列迫使复制
--   整条渠道（含明文 key）；数组 + GIN 后 `groups @> ARRAY['vip']` 直查。
-- =====================================================================
CREATE TABLE IF NOT EXISTS api_channels (
    key           UUID PRIMARY KEY,               -- 渠道唯一 id；route unit 的 channel_key 指向它
    name          TEXT UNIQUE NOT NULL,           -- 渠道名；管理台展示 + 日志冗余
    channel_type  TEXT NOT NULL DEFAULT 'openai', -- 协议族（openai/anthropic/gemini/...）；决定 codec 与 adaptor 选择
    base_url      TEXT NOT NULL DEFAULT '',       -- 上游服务基础地址
    keys          JSONB NOT NULL DEFAULT '[]',    -- 上游密钥数组 [{"index":0,"secret":"..."}]；index 供 route unit 的 key_index 精确取用
    models        JSONB NOT NULL DEFAULT '[]',    -- 模型数组："gpt-4o"=别名直通；{"alias":"...","upstream":"..."}=别名→上游真名映射
    group_name    TEXT NOT NULL DEFAULT 'default',-- [演进 v2 中删除] 旧的单值分组列，见文件尾 ALTER 序列
    priority      INT  NOT NULL DEFAULT 0,        -- 调度优先级，越大越优先；dispatch 先按它分层
    weight        INT  NOT NULL DEFAULT 0,        -- 同层内加权随机的权重
    status        SMALLINT NOT NULL DEFAULT 1,    -- 1=启用 2=手动禁用 3=自动熔断；快照只收 1
    tags          JSONB NOT NULL DEFAULT '[]',    -- 管理台自由标签；数据面不读
    test_model    TEXT,                           -- 渠道探活用模型名（预留）
    remark        TEXT NOT NULL DEFAULT '',       -- 管理员备注
    settings      JSONB NOT NULL DEFAULT '{}',    -- 渠道级覆盖（extra_headers 等）；forward::extra_headers_from_settings 读它
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 旧库补列（新库由上面 CREATE 直接覆盖；列缺失时补齐）
ALTER TABLE api_channels ADD COLUMN IF NOT EXISTS tags JSONB NOT NULL DEFAULT '[]';
ALTER TABLE api_channels ADD COLUMN IF NOT EXISTS settings JSONB NOT NULL DEFAULT '{}';

-- [演进 v2] group_name → groups TEXT[]
ALTER TABLE api_channels ADD COLUMN IF NOT EXISTS groups TEXT[] NOT NULL DEFAULT '{}';
-- 回填：旧单值 → 单元素数组；只在数组为空时填，幂等可重跑
UPDATE api_channels SET groups = ARRAY[group_name]
WHERE group_name IS NOT NULL AND groups = '{}';
ALTER TABLE api_channels DROP COLUMN IF EXISTS group_name;

CREATE INDEX IF NOT EXISTS idx_api_channels_tags   ON api_channels USING GIN (tags);    -- 管理台按标签筛选；数据面不依赖
CREATE INDEX IF NOT EXISTS idx_api_channels_groups ON api_channels USING GIN (groups);  -- 管理台"按组筛渠道"；快照构建不走此索引
CREATE INDEX IF NOT EXISTS idx_api_channels_status ON api_channels (status) WHERE status = 1; -- 快照构建只扫启用行

-- ---------------------------------------------------------------------
-- api_groups — 分组实体：模型白名单（门禁）+ 计费倍率
-- [背景] 旧代码里 ratio 与 model_whitelist 是"死字段"（无人读取）；
--   gate 白名单与计费倍率接线后此表才真正生效。参考：new-api 把这些塞
--   options KV 表，我们用正经实体表以支持管理台按组编辑。
-- =====================================================================
CREATE TABLE IF NOT EXISTS api_groups (
    key             UUID PRIMARY KEY,
    name            TEXT UNIQUE NOT NULL,                  -- 分组 id；api_tokens.group_id / auth_users.group_id 指向它
    ratio           DOUBLE PRECISION NOT NULL DEFAULT 1.0, -- 计费倍率：结算 cost × ratio；写入侧校验 >0 且有限
    model_whitelist JSONB NOT NULL DEFAULT '[]',           -- 组级模型白名单 ["gpt-4","gpt-4*"]；[] = 不限制；支持 * 通配
    remark          TEXT NOT NULL DEFAULT '',
    status          SMALLINT NOT NULL DEFAULT 1,           -- 1=启用；禁用组 → 该组请求在 gate 层整组拒绝
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- 种子行：default 组倍率 1.0
INSERT INTO api_groups (key, name, ratio)
VALUES ('00000000-0000-0000-0000-0000000000d1', 'default', 1.0)
ON CONFLICT (name) DO NOTHING;

-- ---------------------------------------------------------------------
-- api_tokens — 调用方凭证
-- [演进] + allowed_models：token 级模型白名单（对齐 new-api 的
--   model_limits_enabled + model_limits 两列，一个 JSONB 空数组即不限，免开关列）。
-- [热点] used_quota 为缓存态：内存结算后周期 flush；进程重启以 usage_logs 重算。
-- =====================================================================
CREATE TABLE IF NOT EXISTS api_tokens (
    key             UUID PRIMARY KEY,               -- token id（UUID）；日志与结算关联键
    user_key        UUID NOT NULL,                  -- 所属用户 → auth_users.key
    name            TEXT NOT NULL,                  -- 凭证名；日志冗余展示
    key_hash        TEXT UNIQUE NOT NULL,           -- sha256(明文 key)；鉴权唯一查找键
    key_preview     TEXT NOT NULL DEFAULT '',       -- 脱敏展示（前缀+后缀）
    group_id        TEXT,                           -- 生效分组；NULL = 沿用 user 的 group_id
    quota           BIGINT NOT NULL DEFAULT 0,      -- token 级额度上限；内部单位 500_000 = $1；unlimited 时忽略
    unlimited_quota BOOLEAN NOT NULL DEFAULT false, -- true = 不限额
    used_quota      BIGINT NOT NULL DEFAULT 0,      -- 已消耗（缓存态，权威在 usage_logs）
    allowed_models  JSONB NOT NULL DEFAULT '[]',    -- token 级模型白名单；[] = 不限制；支持 * 通配
    expires_at      TIMESTAMPTZ,                    -- 过期时间；NULL = 永不过期
    status          SMALLINT NOT NULL DEFAULT 1,    -- 1=启用；快照只收 1
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
ALTER TABLE api_tokens ADD COLUMN IF NOT EXISTS allowed_models JSONB NOT NULL DEFAULT '[]';
CREATE INDEX IF NOT EXISTS idx_api_tokens_user_key ON api_tokens(user_key); -- 管理台"某用户的 tokens"

-- ---------------------------------------------------------------------
-- auth_users — 账号（登录/注册/管理台；读多写少）
-- 与 new-api users 表的差异：不内联 OAuth id 列（将来独立关联表）、
-- quota 用 BIGINT（new-api 的 int 大额度会溢出）。
-- =====================================================================
CREATE TABLE IF NOT EXISTS auth_users (
    key           UUID PRIMARY KEY,
    username      TEXT UNIQUE NOT NULL,             -- 登录名
    display_name  TEXT NOT NULL DEFAULT '',         -- 昵称
    email         TEXT UNIQUE,                      -- 邮箱；可空
    password_hash TEXT NOT NULL,                    -- 密码哈希
    role          SMALLINT NOT NULL DEFAULT 1,      -- 1=普通 2=管理员
    status        SMALLINT NOT NULL DEFAULT 1,      -- 1=启用
    quota         BIGINT  NOT NULL DEFAULT 0,       -- 用户钱包余额（资金来源层，与 token 额度独立）；500_000 = $1
    used_quota    BIGINT  NOT NULL DEFAULT 0,       -- 累计消耗（缓存态，权威在 usage_logs）
    group_id      TEXT    NOT NULL DEFAULT 'default',-- 默认分组；token.group_id 为空时生效
    auth_version  BIGINT  NOT NULL DEFAULT 1,       -- 改密 +1；refresh token 全量失效判据
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- auth_refresh_tokens — 登录会话（写：每次登录/refresh；过期清理靠定期删）
CREATE TABLE IF NOT EXISTS auth_refresh_tokens (
    sid          UUID PRIMARY KEY,              -- 会话 id
    user_key     UUID NOT NULL,                 -- → auth_users.key
    token_hash   TEXT NOT NULL,                 -- refresh token 哈希
    auth_version BIGINT NOT NULL,               -- 签发时的 auth_version；refresh 时与用户当前值比对，改密即全量失效
    user_agent   TEXT NOT NULL DEFAULT '',      -- 设备标识（会话列表展示）
    ip           TEXT NOT NULL DEFAULT '',      -- 签发 IP
    issued_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ NOT NULL,          -- 硬过期
    revoked_at   TIMESTAMPTZ                    -- 主动登出时间；NULL = 有效
);
