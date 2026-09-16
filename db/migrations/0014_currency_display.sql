-- =====================================================================
-- 0014 货币展示元数据 + 通用换算层 (symbol / kind / precision)
-- =====================================================================
-- kind 语义（0014 引入）：
--   points — 可入账、可扣费的钱包余额货币（进 user_balances；available_i64 计入）
--   fiat   — 仅计价/展示的法定货币（不进 user_balances、不参与扣费/seed）
-- 基准：内部单位 500_000 = $1（pricing.rs 口径）；
--   internal_rate = 1 该货币单位值多少内部单位，两两换算经内部单位中转。

ALTER TABLE currency_defs ADD COLUMN IF NOT EXISTS symbol TEXT NOT NULL DEFAULT '';
ALTER TABLE currency_defs ADD COLUMN IF NOT EXISTS kind TEXT NOT NULL DEFAULT 'points'
    CHECK (kind IN ('points', 'fiat'));
ALTER TABLE currency_defs ADD COLUMN IF NOT EXISTS precision SMALLINT NOT NULL DEFAULT 0;

-- 基准货币 USD：internal_rate 恒为 1（它就是基准本身，管理员不改）。
INSERT INTO currency_defs (code, name, internal_rate, enabled, remark, symbol, kind, precision)
VALUES ('USD', 'US Dollar', 1, true, '基准货币；内部单位 500_000 = $1，rate 恒为 1。', '$', 'fiat', 2)
ON CONFLICT (code) DO NOTHING;

-- CNY：占位参考汇率 0.14（≈¥7.1/$1），仅展示/计价口径，管理员经 /api/currency 可改。
INSERT INTO currency_defs (code, name, internal_rate, enabled, remark, symbol, kind, precision)
VALUES ('CNY', 'Chinese Yuan', 0.14, true, '占位参考汇率，管理员后台可改。', '¥', 'fiat', 2)
ON CONFLICT (code) DO NOTHING;

-- 存量 FREE 显式标为 points（DEFAULT 已覆盖，这里只补展示符号，幂等）。
UPDATE currency_defs SET symbol = 'P', precision = 0
WHERE code = 'FREE' AND symbol = '';
