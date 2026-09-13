-- =====================================================================
-- 0008 充值订单表（用户开单，provider 占位，先手工 settle）
-- =====================================================================

CREATE TABLE IF NOT EXISTS billing_topups (
    key           TEXT PRIMARY KEY,               -- UUID 字符串，如 "550e8400-e29b-41d4-a716-446655440000"
    user_key      UUID NOT NULL,                 -- → auth_users.key
    currency      TEXT NOT NULL,                 -- 对应 currency_defs.code
    amount        BIGINT NOT NULL,               -- 充值金额（该货币单位，不是内部单位）
    state         TEXT NOT NULL DEFAULT 'pending', -- pending | settling | paid | failed | refunded
    provider      TEXT NOT NULL DEFAULT '',      -- 支付渠道占位，如 "epay"
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    settled_at    TIMESTAMPTZ,                    -- 充值到账时间（手工 settle 时写入）
    PRIMARY KEY (key)
);

CREATE INDEX IF NOT EXISTS idx_billing_topups_user ON billing_topups(user_key);
CREATE INDEX IF NOT EXISTS idx_billing_topups_state ON billing_topups(state);

COMMENT ON TABLE billing_topups IS '充值订单，provider 占位，由 admin-billing 手工 settle 入金';
COMMENT ON COLUMN billing_topups.state IS '状态机: pending(未支付) → settling(结算中,CAS占位) → paid/failed/refunded(终态)';
COMMENT ON COLUMN billing_topups.provider IS '支付渠道，如 "epay" | "stripe" | "creem"';