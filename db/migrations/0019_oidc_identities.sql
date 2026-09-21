-- 0019 OIDC 登录支持：外部身份与本站用户绑定
-- 包含三张表，全部使用 IF NOT EXISTS 以保证迁移的幂等性
-- 1. identity_providers — OIDC 各 provider 配置（admin 管理，不同的 Google/GitHub/Keycloak 等都能在这里配置）
--    slug 作为 URL 路径段，如 /oidc/google/authorize，issuer_url 用于 OIDC discovery。
--    enabled 默认 false — 配置好再开；client_secret 由 admin 写入明文，接口对外屏蔽。
--    scopes 存储为 TEXT[]，方便管理员统一调整；default_role 控制首次注册角色，auto_create_user 控制自动建号行为。
CREATE TABLE IF NOT EXISTS identity_providers (
    slug              TEXT PRIMARY KEY,
    display_name      TEXT NOT NULL,
    issuer_url        TEXT NOT NULL,
    client_id         TEXT NOT NULL,
    client_secret     TEXT NOT NULL,
    redirect_uri      TEXT NOT NULL,
    scopes            TEXT[] NOT NULL DEFAULT ARRAY['openid','email','profile'],
    auto_create_user  BOOLEAN NOT NULL DEFAULT true,
    default_role      SMALLINT NOT NULL DEFAULT 1,
    enabled           BOOLEAN NOT NULL DEFAULT false,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 2. user_identities — 外部身份（provider + subject）到本站用户的映射表
--    subject 是 ID token 里的 sub 字段；一个用户可以在不同 provider 下拥有不同 subject。
--    email 冗余存储，方便未绑定外部身份时的查找。
--    last_login_at 方便日志分析。
--    复合主键确保一 provider+subject 映射唯一一行。
CREATE TABLE IF NOT EXISTS user_identities (
    provider_slug  TEXT NOT NULL REFERENCES identity_providers(slug) ON DELETE CASCADE,
    subject        TEXT NOT NULL,
    user_key       UUID NOT NULL REFERENCES auth_users(key) ON DELETE CASCADE,
    email          TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_login_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (provider_slug, subject)
);
CREATE INDEX IF NOT EXISTS idx_user_identities_user_key ON user_identities(user_key);

-- 3. oidc_auth_states — state/nonce/PKCE 认证参数临时存储（防重放，幂等支持）
--    state 是跨端点验证码；nonce 用于验证 ID token；pkce_verifier 用于 PKCE 流程。
--    expires_at 是有效期截止点，定期归档清理。
CREATE TABLE IF NOT EXISTS oidc_auth_states (
    state           TEXT PRIMARY KEY,
    provider_slug   TEXT NOT NULL REFERENCES identity_providers(slug) ON DELETE CASCADE,
    nonce           TEXT NOT NULL,
    pkce_verifier   TEXT NOT NULL,
    redirect_after  TEXT,
    expires_at      TIMESTAMPTZ NOT NULL
);