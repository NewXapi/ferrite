-- =====================================================================
-- 0009 登录会话表（auth::service 全程读写；历史由 ensure_table 建，
-- #116 退役 ensure_table 时漏入库 —— 全新库 login 直接
-- `relation "auth_user_sessions" does not exist`，e2e wire-contract 实锤）。
-- 列定义对齐存量库（ferrite dev）实际 schema，保证迁移后行为一致。
-- =====================================================================

CREATE TABLE IF NOT EXISTS auth_user_sessions (
    sid          UUID PRIMARY KEY,               -- 会话 id（登录签发，吊销/续期按它定位）
    user_key     UUID NOT NULL,                  -- → auth_users.key
    user_agent   TEXT NOT NULL DEFAULT '',       -- 登录 UA（会话面板展示）
    ip           TEXT NOT NULL DEFAULT '',       -- 登录 IP
    login_method TEXT NOT NULL DEFAULT 'password', -- password | ...
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_active  TIMESTAMPTZ NOT NULL DEFAULT now(), -- 活跃续期（service 心跳 UPDATE）
    expires_at   TIMESTAMPTZ NOT NULL,           -- 绝对过期（refresh 轮换时刷新）
    revoked_at   TIMESTAMPTZ                     -- 吊销时刻；NULL = 有效（登出/全端下线置值）
);

CREATE INDEX IF NOT EXISTS idx_auth_user_sessions_user
    ON auth_user_sessions(user_key, created_at);

COMMENT ON TABLE auth_user_sessions IS '登录会话（sid 定位；revoked_at IS NULL = 有效）';
