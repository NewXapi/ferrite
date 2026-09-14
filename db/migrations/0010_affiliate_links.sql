-- =====================================================================
-- 0010 拉人归属关系 + 奖励入账审计（affiliate 域，支撑真实统计）
-- =====================================================================
-- 读写场景：
-- - 写：绑定归属（bind_inviter，每被邀人一行，注册/后台补绑时）；领奖
--   （reward_invite_referral，审计行与 user_balances 入金同事务，一次一行）。
-- - 读：user_overview 的 COUNT/SUM 聚合（读多写少，用户端总览 + 管理台）。
-- - 行数量级：links ≈ 有邀请归属的用户数（≤ 用户数）；rewards ≈ 领奖次数
--   + 活动奖励次数（同为用户数级别，低频写）。

-- affiliate_links — 邀请归属（读多写少；行数 ≤ 用户数）
-- PK 用 invitee_key（不是 (inviter,invitee) 复合键）：一个被邀人只能归属一个
-- 邀请人，先到先得——防止同一被邀人被多人重复绑定、触发多份 invite 奖励。
CREATE TABLE IF NOT EXISTS affiliate_links (
    inviter_key  UUID NOT NULL,               -- → auth_users.key（邀请人）
    invitee_key  UUID NOT NULL,               -- → auth_users.key（被邀人）
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (invitee_key)
);
CREATE INDEX IF NOT EXISTS idx_affiliate_links_inviter ON affiliate_links(inviter_key);

-- affiliate_rewards — 奖励入账审计（低频写；行数 ≈ 领奖次数 + 活动发奖次数）
CREATE TABLE IF NOT EXISTS affiliate_rewards (
    key          UUID PRIMARY KEY,
    inviter_key  UUID NOT NULL,               -- → auth_users.key
    invitee_key  UUID,                        -- 触发来源被邀人；活动奖励可空
    kind         TEXT NOT NULL,               -- 'invite' | 'activity' | ...
    amount       BIGINT NOT NULL,             -- 奖励额（FREE 货币单位）
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_affiliate_rewards_inviter ON affiliate_rewards(inviter_key);
-- DB 级幂等护栏：同一被邀人的 'invite' 奖励全局只允许一行。
-- 应用层「先插审计、撞约束即已领」单语句判定，无需 SELECT-then-INSERT
-- （并发双领奖 TOCTOU 也只有第一条能入金）。活动奖励（kind≠'invite'
-- 或 invitee_key IS NULL）不受本约束限制。
CREATE UNIQUE INDEX IF NOT EXISTS uq_affiliate_rewards_invite
    ON affiliate_rewards(invitee_key) WHERE kind = 'invite';

COMMENT ON TABLE affiliate_links IS '邀请归属关系：invitee_key PK = 一人一主，防重复绑定/重复领奖';
COMMENT ON COLUMN affiliate_rewards.kind IS '''invite''(拉人领奖,每被邀人限一次) | ''activity''(运营活动,可空 invitee)';
