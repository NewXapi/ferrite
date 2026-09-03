# `crates/admin-api/auth`

## 职责

admin-api 控制面的人认证 — 登录 / 注册 / refresh / logout / self。

参考：`todo/admin-api/README.md`（new-api 调查精简版）。

## 模块

- `password` — argon2id，PHC 字符串存 `auth_users.password_hash`。
- `jwt` — HS256。access 15min / refresh 7d。claims: `sub`/`role`/`auth_version`/`sid`/`exp`。
- `service::AuthService` — 业务逻辑；直连 `sqlx::PgPool`，**不走 store trait**（loose 表阶段）。
- `ddl` — `auth_users` / `auth_refresh_tokens` 启动时 `IF NOT EXISTS` 建表。
- `routes::router(pool)` — axum 子路由，5 个端点。

## 端点

| 方法   | 路径                  | 说明                              |
|--------|-----------------------|-----------------------------------|
| POST   | `/api/user/login`     | username+password → JWT + refresh |
| POST   | `/api/user/register`  | 自注册（本期开启）                |
| POST   | `/api/user/refresh`   | refresh → 新 access + 新 refresh（旧 sid 吊销）|
| POST   | `/api/user/logout`    | refresh → 吊销 sid                |
| GET    | `/api/user/self`      | 当前用户（Bearer access）         |

## 表（loose，无 FK / outbox）

```sql
CREATE TABLE IF NOT EXISTS auth_users (
    key           UUID PRIMARY KEY,
    username      TEXT UNIQUE NOT NULL,
    display_name  TEXT NOT NULL DEFAULT '',
    email         TEXT UNIQUE,
    password_hash TEXT NOT NULL,                  -- argon2id PHC
    role          SMALLINT NOT NULL DEFAULT 1,    -- 1=user 10=admin 100=root
    status        SMALLINT NOT NULL DEFAULT 1,    -- 1=enabled 2=disabled
    quota         BIGINT  NOT NULL DEFAULT 0,
    used_quota    BIGINT  NOT NULL DEFAULT 0,
    group_id      TEXT    NOT NULL DEFAULT 'default',
    auth_version  BIGINT  NOT NULL DEFAULT 1,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS auth_refresh_tokens (
    sid          UUID PRIMARY KEY,
    user_key     UUID NOT NULL,
    token_hash   TEXT NOT NULL,                   -- HMAC(secret) hex
    user_agent   TEXT NOT NULL DEFAULT '',
    ip           TEXT NOT NULL DEFAULT '',
    issued_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ NOT NULL,
    revoked_at   TIMESTAMPTZ
);
```

## 不做（后续会话）

- 2FA / passkey / OAuth
- 邮箱验证 / 找回密码
- 改密（`auth_version` 字段已就位，但 handler 未实现）
- 多设备 session 列表
- 30s refresh 重放窗口（new-api 行为是给重试 30s 宽限；MVP 直接吊销旧）
- 邀请码 / AffCode
- 审计日志

## 验证

```sh
# 1) 单元测试（不需 PG）
cargo test -p auth

# 2) 集成测试（需 PG）
DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite \
    cargo test -p auth --test integration -- --ignored --nocapture

# 3) 编译
cargo check -p auth -p admin-api-router
cargo clippy -p auth -p admin-api-router --all-targets
```

## 集成

`apps/api` 后续接入：

```rust
// apps/api/src/main.rs
let admin = admin_api_router::router(pool.clone()).await;
let app = gateway.router().merge(admin);
```
