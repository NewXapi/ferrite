# auth

本站账号认证 + OIDC 外部身份解析。

## 职责

本地账号的登录/注册/refresh/logout/self（Argon2id + JWT 刷新轮换），以及把 OIDC
外部身份 `(provider_slug, subject, email)` 解析成本站用户三元组。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/service.rs` | `AuthService` 业务逻辑，直连 `sqlx::PgPool` 读写 `auth_*` 平表 |
| `src/jwt.rs` | HS256，access 15min / refresh 7d；claims: `sub`/`role`/`auth_version`/`sid`/`exp` |
| `src/password.rs` | Argon2id，PHC 字符串存 `password_hash` |
| `src/routes.rs` | axum 子路由；`router(pool)` / `router_with_svc(Arc<AuthService>)` 两个组装入口 |
| `src/identity.rs` | OIDC 外部身份 → 本站用户解析（单事务三级查找） |
| `src/error.rs` | `AuthError` + status/code 映射 |
| `src/lib.rs` | crate 导出面 |

## 关键安全语义

- **`email_verified=false` 时禁止按 email 匹配已有账号**——未验证邮箱可被任意人在
  IdP 侧写入 claim，按它 Bind 等于账号劫持。只能走 subject 精确命中或新建账号。
- OIDC 建号时 `default_role.min(1)` 夹到普通用户：provider 配错不该能造出 admin/root。
- `role` 从 SMALLINT 读回用 `u16::try_from(...).unwrap_or(1)`，**不能** `as u16`
  （`-1 as u16` = 65535 = 越权成 root）。
- 登录防用户枚举（dummy-hash 等时延）；禁用用户密码正确时返 `USER_DISABLED`。
- `ADMIN_ROLE_THRESHOLD = 10`。
- 全程单事务 + 每级 `FOR UPDATE`，防并发首登重复建号。

## 端点（本地账号 9 + OIDC 由 auth-oidc 提供）

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/api/user/login` | username+password → JWT + refresh |
| POST | `/api/user/register` | 自注册 |
| POST | `/api/user/refresh` | refresh → 新 access + 新 refresh（旧 sid 吊销） |
| POST | `/api/user/logout` | refresh → 吊销 sid |
| GET/PUT/DELETE | `/api/user/self` | 当前用户 / 改昵称改密 / 注销 |
| GET | `/api/user` | admin 用户列表 |
| GET | `/api/user/search` | admin 搜索（ILIKE 前 20 条） |
| GET | `/api/user/{key}` | admin 单查 |
| POST | `/api/user/manage` | enable/disable/set_role/adjust_quota/reset_password |

## 验收

```bash
cargo check -p auth
cargo test -p auth                         # CI
```
