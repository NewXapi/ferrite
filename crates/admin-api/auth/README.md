# `crates/admin-api/auth`

## 职责

admin-api 控制面的人认证 — 登录 / 注册 / refresh / logout / self / admin 用户管理（9 端点）。

参考：`todo/admin-api/README.md`（new-api 调查精简版）。

## 模块

- `password` — argon2id，PHC 字符串存 `auth_users.password_hash`。
- `jwt` — HS256。access 15min / refresh 7d。claims: `sub`/`role`/`auth_version`/`sid`/`exp`。
- `service::AuthService` — 业务逻辑；直连 `sqlx::PgPool`，**不走 store trait**（loose 表阶段）。
  `new()` 返回 `Result`（jwt_secret ≥32B 校验，不再 assert panic）。
- `ddl` — `auth_users` / `auth_refresh_tokens` 启动时 `IF NOT EXISTS` 建表。
- `routes` — axum 子路由。组装入口两个：`router(pool)`（读 env）/
  `router_with_svc(Arc<AuthService>)`（admin-api-router 五域聚合时共享实例用）。
  导出 `bearer_user()` 与 `ADMIN_ROLE_THRESHOLD` 供兄弟域复用。

## 端点（9）

| 方法   | 路径                  | 说明                              |
|--------|-----------------------|-----------------------------------|
| POST   | `/api/user/login`     | username+password → JWT + refresh |
| POST   | `/api/user/register`  | 自注册（本期开启）                |
| POST   | `/api/user/refresh`   | refresh → 新 access + 新 refresh（旧 sid 吊销，rows_affected 校验防并发重放）|
| POST   | `/api/user/logout`    | refresh → 吊销 sid                |
| GET    | `/api/user/self`      | 当前用户（Bearer access）         |
| PUT    | `/api/user/self`      | 改昵称 / 改密（原子改密 WHERE 旧 hash，auth_version++ 全端失效）|
| DELETE | `/api/user/self`      | 注销（硬删 + refresh 清理）       |
| GET    | `/api/user`           | admin 用户列表 (search/page/size) |
| GET    | `/api/user/search`    | admin 搜索（ILIKE，前 20 条）     |
| GET    | `/api/user/{key}`     | admin 单查                        |
| POST   | `/api/user/manage`    | admin: enable/disable/set_role/adjust_quota/reset_password（`ManageUserAction` enum）|

安全语义：登录防用户枚举（dummy-hash 等时延）、禁用用户密码正确时返 `USER_DISABLED`（与 refresh 一致）、
密码 8..=128 字节、username ≤32 / display_name ≤64、`ADMIN_ROLE_THRESHOLD = 10`。

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
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS auth_refresh_tokens (
    sid          UUID PRIMARY KEY,
    user_key     UUID NOT NULL,
    token_hash   TEXT NOT NULL,                   -- HMAC(secret) hex
    auth_version BIGINT NOT NULL,                 -- 发放时的用户 auth_version，refresh 时比对
    user_agent   TEXT NOT NULL DEFAULT '',
    ip           TEXT NOT NULL DEFAULT '',
    issued_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ NOT NULL,
    revoked_at   TIMESTAMPTZ
);
```

`auth_refresh_tokens.auth_version` 是改密失效机制的关键：refresh 时与用户当前值比对，
不一致即失效（测试曾抓到 JOIN 读当前值的 bug，已改为发放时落列）。

## 不做（后续会话）

- 2FA / passkey / OAuth / 邮箱验证 / 找回密码
- 多设备 session 列表（`auth_refresh_tokens` 有数据基础，接口未暴露）
- 30s refresh 重放窗口（new-api 给重试 30s 宽限；MVP 直接吊销旧）
- 邀请码 / AffCode / 审计日志

## 验证

```sh
# 1) 单元测试（不需 PG）
cargo test -p auth

# 2) 集成测试（需 PG）
DATABASE_URL=postgres://<user>:<pass>@127.0.0.1:5433/<db> \
    cpulimit -l 70 -i -- cargo test -p auth --test integration -- --ignored --nocapture

# 3) 编译
cargo check -p auth -p admin-api-router
cargo clippy -p auth -p admin-api-router --all-targets
```

## 集成

`apps/api` 后续接入（`admin-api-router` 已聚合五域，一行挂载）：

```rust
// apps/api/src/main.rs
let admin = admin_api_router::router(pool.clone()).await?;
let app = gateway.router().merge(admin);
```
