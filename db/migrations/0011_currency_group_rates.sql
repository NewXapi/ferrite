-- =====================================================================
-- 0011 货币组差异化倍率（currency_defs.group_rates）
-- =====================================================================
-- currency_defs 加组倍率：组名 → 倍率（NULL/缺组 = 1.0）。
-- 例：{"vip": 0.8} = vip 组用户的 FREE 货币按 0.8 倍计价（折扣语义）。
-- 生效公式（wallet.rs available_i64/deduct_by_cost_group 同口径）：
--   内部单位 = amount × internal_rate × group_multiplier
-- 现有表 ADD COLUMN，无需重建；`{}` 缺省 = 全组 1.0（行为不变）。
ALTER TABLE currency_defs ADD COLUMN IF NOT EXISTS group_rates JSONB NOT NULL DEFAULT '{}';
