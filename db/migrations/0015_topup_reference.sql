-- =====================================================================
-- 0015 充值订单加外部支付引用列
-- =====================================================================
-- epay 等真渠道开单时，商户侧外部单号 / 支付引用需要随订单落库，
-- 供对账与回调核验；manual 单无外部引用，留 NULL。

ALTER TABLE billing_topups ADD COLUMN IF NOT EXISTS reference TEXT;

COMMENT ON COLUMN billing_topups.reference IS '外部支付引用（epay out_trade_no 等；manual 单为 NULL）';
