//! Loose 表 DDL — 启动时跑一次。
//!
//! 无 FK / outbox / sync_meta。MVP 用平表顶住，等数据稳定再迁到 contract::records。

use sqlx::PgPool;

const DDL: &str = r#"
CREATE TABLE IF NOT EXISTS auth_users (
    key           UUID PRIMARY KEY,
    username      TEXT UNIQUE NOT NULL,
    display_name  TEXT NOT NULL DEFAULT '',
    email         TEXT UNIQUE,
    password_hash TEXT NOT NULL,
    role          SMALLINT NOT NULL DEFAULT 1,
    status        SMALLINT NOT NULL DEFAULT 1,
    quota         BIGINT  NOT NULL DEFAULT 0,
    used_quota    BIGINT  NOT NULL DEFAULT 0,
    group_id      TEXT    NOT NULL DEFAULT 'default',
    auth_version  BIGINT  NOT NULL DEFAULT 1,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS auth_refresh_tokens (
    sid          UUID PRIMARY KEY,
    user_key     UUID NOT NULL,
    token_hash   TEXT NOT NULL,
    user_agent   TEXT NOT NULL DEFAULT '',
    ip           TEXT NOT NULL DEFAULT '',
    issued_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ NOT NULL,
    revoked_at   TIMESTAMPTZ
);
"#;

pub async fn run(pool: &PgPool) -> Result<(), sqlx::Error> {
    // 简单查询协议 — 多语句 DDL 一次性发，IF NOT EXISTS 让并发首次启动收敛。
    sqlx::raw_sql(DDL).execute(pool).await?;
    Ok(())
}
