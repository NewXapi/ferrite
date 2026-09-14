-- =====================================================================
-- 0012 奖励冻结（user_balances.frozen_amount + affiliate_rewards.frozen_until）
-- =====================================================================
-- 冻结语义：
-- - `available = amount - frozen_amount`（冻结部分不进可用/扣减口径）。
-- - 冻结随奖励行发生：`credit_reward_frozen` 把 amount 与 frozen_amount 同加
--   （可用不变），到期时间记在 `affiliate_rewards.frozen_until`（按奖励行粒度，
--   非按用户）；thaw_frozen JOIN 到期行把 frozen_amount 搬回可用并清空到期标记。
-- 现有行 DEFAULT 0 / NULL = 无冻结，行为不变。
ALTER TABLE user_balances ADD COLUMN IF NOT EXISTS frozen_amount BIGINT NOT NULL DEFAULT 0;
ALTER TABLE affiliate_rewards ADD COLUMN IF NOT EXISTS frozen_until TIMESTAMPTZ;

COMMENT ON COLUMN user_balances.frozen_amount IS '冻结额度（货币单位）；available = amount - frozen_amount，冻结部分不可扣';
COMMENT ON COLUMN affiliate_rewards.frozen_until IS '奖励冻结到期时间；NULL = 未冻结/已解冻（thaw 后清空，幂等标记）';
