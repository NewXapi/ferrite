-- =====================================================================
-- 0015 auth_users 分组单值 → 多值数组
-- ---------------------------------------------------------------------
-- 动因：管理台「生效分组」改多选。一个用户可同时属多个分组（vip + trial），
-- 单值 group_id 迫使管理员只选一个；数组后筛选/编辑都能多选。
-- 语义：groups[1] 为「生效分组」——token 未显式设组时的回落值
-- （gate/src/chain.rs 的 token.group ?? user.group 链路），也是
-- billing 组倍率（currency_defs.group_rates）的取数键。
--
-- 与 0001 的渠道演进（group_name → groups TEXT[]）同型：
--   - 旧列 group_id 保留为死列（不删，避免锁表；读取侧全部切到 groups）；
--   - 回填后老数据的生效分组不丢；
--   - GIN 索引支撑管理台「按组筛用户」。
-- 幂等：ADD COLUMN IF NOT EXISTS + 回填只在 groups = '{}' 时执行。
-- =====================================================================

ALTER TABLE auth_users ADD COLUMN IF NOT EXISTS groups TEXT[] NOT NULL DEFAULT '{}';

UPDATE auth_users
SET groups = ARRAY[group_id]
WHERE group_id IS NOT NULL
  AND group_id <> ''
  AND groups = '{}';

CREATE INDEX IF NOT EXISTS idx_auth_users_groups
    ON auth_users USING GIN (groups);
