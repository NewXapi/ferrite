-- =====================================================================
-- 0007 货币定义 + 用户货币余额
-- =====================================================================

-- currency_defs — 货币注册表（读多写极少；行数：个位）
CREATE TABLE IF NOT EXISTS currency_defs (
    code           TEXT PRIMARY KEY,              -- 'FREE' | 'PAID' | 'GIFT' | ...
    name           TEXT NOT NULL,                -- 展示名（管理台）
    internal_rate  DOUBLE PRECISION NOT NULL DEFAULT 1, -- 1 单位 = internal_rate 内部单位（500_000=$1 语义）
    enabled        BOOLEAN NOT NULL DEFAULT true, -- false = 新入账/查询忽略该货币
    remark         TEXT NOT NULL DEFAULT '',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- seed 默认货币
INSERT INTO currency_defs (code, name, internal_rate, enabled)
VALUES ('FREE', 'Free Points', 1, true)
ON CONFLICT (code) DO NOTHING;

-- user_balances — 用户货币余额（读多写少；行数 = 用户数 × 启用货币数）
CREATE TABLE IF NOT EXISTS user_balances (
    user_key       UUID NOT NULL,               -- → auth_users.key
    currency_code  TEXT NOT NULL,               -- → currency_defs.code
    amount         BIGINT NOT NULL DEFAULT 0,   -- 该货币余额（该货币自己的单位，不是内部单位）
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_key, currency_code)
);
CREATE INDEX IF NOT EXISTS idx_user_balances_user ON user_balances(user_key);

-- 用户注册时 seed 当前所有启用货币（由 admin-billing/registration hook 触发）