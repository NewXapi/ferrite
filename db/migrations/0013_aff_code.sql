-- =====================================================================
-- 0013 邀请短码（auth_users.aff_code）
-- =====================================================================
-- 语义：
-- - aff_code = 邀请短码，注册链接 `?invite=<aff_code>`（参数名沿用 invite：
--   落地页 admin-page-auth 只读 invite，换名要二改前端；解析侧双格式，
--   先 UUID 兼容旧链接，再 aff_code 查表——见 currency::resolve_invite_code）。
-- - 稀疏唯一：NULL 允许多行（未生成/老流程的存量态），非 NULL 全局唯一；
--   应用侧生成撞码重试（currency::generate_aff_code），唯一索引是最后护栏。
ALTER TABLE auth_users ADD COLUMN IF NOT EXISTS aff_code TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS auth_users_aff_code_uidx
    ON auth_users (aff_code) WHERE aff_code IS NOT NULL;

-- 存量回填：逐行生成 8 位 hex 短码，撞唯一索引换码重试（最多 16 次）。
-- ponytail: substr(md5(random())) 的 8 hex ≈ 43 亿空间，万级用户内生日冲突
-- 可忽略；唯一索引 + 重试兜底，不引额外序列或应用侧回填脚本。循环只动
-- aff_code IS NULL 的行，重跑（迁移幂等）时不覆盖已生成的码。
-- 注意与运行时 generate_aff_code（base62 6 位）字母表/长度不同：这是有意
-- 的——两者只要求"随机 + 唯一"，走同一条 resolve_invite_code 解析路径，
-- 格式差异对功能无影响；存量码不重写。
DO $$
DECLARE
    rec     RECORD;
    code    TEXT;
    attempt INT;
BEGIN
    FOR rec IN SELECT key FROM auth_users WHERE aff_code IS NULL LOOP
        attempt := 0;
        LOOP
            code := substr(md5(random()::text), 1, 8);
            BEGIN
                UPDATE auth_users SET aff_code = code
                WHERE key = rec.key AND aff_code IS NULL;
                EXIT WHEN FOUND;
            EXCEPTION WHEN unique_violation THEN
                attempt := attempt + 1;
                IF attempt > 16 THEN
                    RAISE EXCEPTION 'aff_code backfill: 16 retries exhausted for user %', rec.key;
                END IF;
            END;
        END LOOP;
    END LOOP;
END $$;

COMMENT ON COLUMN auth_users.aff_code IS '邀请短码；NULL = 未生成，注册链接 ?invite=<aff_code>，非 NULL 全局唯一（auth_users_aff_code_uidx）';
