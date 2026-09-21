# auth-oidc

OIDC / OAuth 2.0 第三方登录的**协议层**：provider 注册表、state 一次性存储、授权码 + PKCE 流程、ID token 验签。

不含 axum 端点、admin CRUD 与前端按钮——那些属于组装层（`apps/api`）与后续 PR。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/lib.rs` |  crate 导出面：`flow` / `provider` / `store` |
| `src/provider.rs` | `IdentityProvider`（对应 `identity_providers` 表）+ `ProviderRegistry`（`ArcSwap` 热加载）+ `OidcError` |
| `src/store.rs` | `AuthStateStore` —— `oidc_auth_states` 表薄封装，`insert` / `take`（取出即删防重放）+ `new_state()` 随机值生成 |
| `src/flow.rs` | `authorize_url`（拼授权 URL）+ `exchange_and_verify`（换 token、验 claims、验 `at_hash`） |

## 安全设计要点

- **JWKS 轮换、nonce/state 校验、PKCE、签名验证全部委托 `openidconnect` crate**，本 crate 不手写密码学。HTTP client 用 `openidconnect::reqwest` 并显式关闭重定向（SSRF 防护）。
- **state 取出即删**：`take()` 是 `DELETE ... RETURNING` 单语句，原子且防重放；跨 provider 的 state 额外校验 `provider_slug`。
- **email 劫持防线在上游**：`crates/api/auth/src/identity.rs` 在 `email_verified=false` 时禁止按 email 匹配已有账号。
- **PKCE verifier 长度**：RFC 7636 要求 43-128 字符，`from_code_verifier_sha256` 对不合规长度会 panic——`new_state()` 用两个 uuid `simple()` 拼接成 64 字符，不自造随机源。

## 依赖方向

纯 leaf crate：只依赖 `sqlx`（PG）、`openidconnect`、`arc-swap`、`uuid`。被 `apps/api` 组装，不依赖其他 `crates/api/*`。

## 验证

```bash
cargo check -p auth-oidc
```

测试尚未补充：`flow.rs` 的完整授权码流程需要 mock IdP（`wiremock` 或本地 issuer），计划在 axum 端点 PR 一并落地。
