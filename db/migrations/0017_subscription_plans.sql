-- =====================================================================
-- 0017 subscription_plans — 可购买的订阅产品（管理台「订阅」页后端）
-- ---------------------------------------------------------------------
-- 动因：管理台订阅页需要真实后端 CRUD。此前 contract 已定义
-- SubscriptionPlanRecord / SubscriptionDto / SubscriptionUpsertRequest，
-- 但无表、无服务——套餐只能写在配置里，运营改价/上下架要发版。
--
-- 设计取舍：
-- - key UUID PK：沿用 0001/0005/0010 的惯例由应用侧 Uuid::new_v4() 生成
--   （仓库迁移从不依赖 gen_random_uuid/pgcrypto，此处不破例）。
-- - price TEXT 存 NUMERIC 语义：JSON 全程传字符串避免 f64 浮点误差
--   （同 SubscriptionPlanRecord.price 口径，如 "9.9"）。
-- - quota BIGINT 内部单位（500_000 = $1）：与 user_balances / currency
--   同口径；API 边界把 f64 展示值 ×500_000 四舍五入后入库。
-- - name 唯一：upsert 的自然键——`ON CONFLICT (name) DO UPDATE` 必须有
--   唯一约束才能工作，同时是下面 seed 的幂等目标。
-- - sort_order：SubscriptionPlanRecord 没有但 SubscriptionDto 有，前端
--   列表排序位次靠它；新行取 MAX+1，upsert 命中既有行时不覆盖位次。
-- - duration_days/max_purchases 用 INT（可空语义对齐 record 的 Option）：
--   max_purchases NULL = 无限购次数。
--
-- 工作负载：套餐行数个位级（运营手工维护），list 全量按 sort_order 排序；
-- 写都是 admin 单条 upsert，无热路径竞争，不需要分页/缓存/分区。
-- 幂等：CREATE TABLE/INDEX IF NOT EXISTS + seed ON CONFLICT DO NOTHING，
-- 对「已被建过表的存量库」与全新库都安全（db-bootstrap 约定）。
-- =====================================================================

CREATE TABLE IF NOT EXISTS subscription_plans (
    key            UUID PRIMARY KEY,               -- 应用侧 Uuid::new_v4() 生成（DELETE /api/subscriptions/{key} 的定位符）
    name           TEXT NOT NULL,                  -- 唯一自然键（subscription_plans_name_uidx）
    price          TEXT NOT NULL,                  -- NUMERIC 语义字符串，如 "9.9"；JSON 传字符串避免浮点误差
    currency       TEXT NOT NULL DEFAULT 'CNY',    -- "CNY" | "USD"，计价展示用
    duration_days  INT NOT NULL,                   -- 订阅时长（天）→ SubscriptionDto.periodVal
    quota          BIGINT NOT NULL,                -- 内部单位（500_000 = $1）
    upgrade_group  TEXT,                           -- 购买后升级到的分组；NULL = 不变
    max_purchases  INT,                            -- 每用户可购次数上限；NULL = 无限
    enabled        BOOLEAN NOT NULL DEFAULT true,  -- 上下架
    sort_order     INT NOT NULL DEFAULT 0,         -- 列表排序位次（小在前）
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS subscription_plans_name_uidx
    ON subscription_plans (name);

CREATE INDEX IF NOT EXISTS idx_subscription_plans_enabled_sort
    ON subscription_plans (enabled, sort_order);

COMMENT ON TABLE subscription_plans IS '可购买的订阅产品（价格/时长/额度/升级组），admin-billing 订阅页 CRUD';
COMMENT ON COLUMN subscription_plans.price IS 'NUMERIC 语义字符串（JSON 传字符串避免浮点误差），应用侧 parse 校验非法值';
COMMENT ON COLUMN subscription_plans.quota IS '内部单位（500_000 = $1）；API 边界 f64 展示值 ×500_000 四舍五入入库';
COMMENT ON COLUMN subscription_plans.sort_order IS '列表排序位次（小在前）；SubscriptionDto.sortOrder 口径，新行 MAX+1';
COMMENT ON COLUMN subscription_plans.upgrade_group IS '购买后用户升级到的分组；NULL = 不变';
COMMENT ON COLUMN subscription_plans.max_purchases IS '每用户可购次数上限；NULL = 无限';

-- 示例套餐：免费体验 30 天 + $1 额度（quota 500_000 = 500_000 内部单位）。
-- key 固定不随机，方便排查；ON CONFLICT (name) DO NOTHING 使重跑迁移幂等。
-- key 须是合规的 8-4-4-4-12 hex：早先末段写成 0000000017（10 位）会被 PG
-- 拒成 "invalid input syntax for type uuid"，迁移 17 整体失败、后端起不来。
INSERT INTO subscription_plans
    (key, name, price, currency, duration_days, quota, upgrade_group, max_purchases, enabled, sort_order)
VALUES ('e0f30000-0000-4b17-8f3f-000000000017', '免费体验', '0', 'CNY', 30, 500000, NULL, 1, true, 0)
ON CONFLICT (name) DO NOTHING;
