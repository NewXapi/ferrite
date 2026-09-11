-- =====================================================================
-- 0005 运营配置 + 兑换码 + 出口节点
-- ---------------------------------------------------------------------
-- options             — 运行时运营配置 KV（注册开关、新用户额度等）；
--                       网关运营参数（限流上限/健康参数）后续也落这里。
-- billing_redemptions — 兑换码（哈希存储，明文码只出现一次不落库）。
-- proxy_nodes         — 出口节点池（#108 起接入数据面热更）；凭据内嵌在 url。
-- =====================================================================

-- options — 运营配置 KV（读：启动快照+管理台；写：管理台；行数：几十）
CREATE TABLE IF NOT EXISTS options (
    key        TEXT PRIMARY KEY,                 -- 配置键（site.* 命名空间）
    value      JSONB NOT NULL,                   -- 配置值（写入侧按 OptionSpec 校验）
    updated_by UUID,                             -- 最近修改人 → auth_users.key；系统种子为 NULL
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO options (key, value) VALUES ('site.registration_enabled', 'true')
    ON CONFLICT (key) DO NOTHING;
INSERT INTO options (key, value) VALUES ('site.quota_new_user', '0')
    ON CONFLICT (key) DO NOTHING;

-- billing_redemptions — 兑换码（读：兑换时按 code_hash 查；写：管理台生成；行数千级）
CREATE TABLE IF NOT EXISTS billing_redemptions (
    key          UUID PRIMARY KEY,
    code_hash    TEXT UNIQUE NOT NULL,           -- sha256(明文兑换码)；明文只在生成响应里出现一次
    code_preview TEXT NOT NULL DEFAULT '',       -- 脱敏展示（前缀+后缀）
    quota        BIGINT NOT NULL,                -- 兑换额度；内部单位 500_000 = $1
    status       SMALLINT NOT NULL DEFAULT 1,    -- 1=未兑换 2=已兑换（已兑后 redeemed_* 有值）
    redeemed_by  UUID,                           -- 兑换人 → auth_users.key；未兑为 NULL
    redeemed_at  TIMESTAMPTZ,                    -- 兑换时间；未兑为 NULL
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_billing_redemptions_status ON billing_redemptions(status);

-- proxy_nodes — 出口节点池（读：启动+热更快照；写：管理台；行数：十级）
CREATE TABLE IF NOT EXISTS proxy_nodes (
    key            UUID PRIMARY KEY,
    name           TEXT NOT NULL DEFAULT '',     -- 节点名（管理台展示）
    url            TEXT NOT NULL,                -- 节点地址：http(s):// socks5(h):// ss:// trojan:// vless:// vmess://；凭据内嵌
    channel_keys   JSONB NOT NULL DEFAULT '[]',  -- 绑定的渠道名数组（空=全部渠道可用）
    priority       INT  NOT NULL DEFAULT 0,      -- 调度优先级，越大越优先
    enabled        BOOL NOT NULL DEFAULT true,   -- false = 快照剔除
    remark         TEXT NOT NULL DEFAULT '',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_proxy_nodes_enabled ON proxy_nodes (enabled);
