# `crates/admin-api/router`

聚合 admin-api 子域的 axum Router，供 `apps/api::main` 一次性挂载。

当前聚合**五域**：

| 子域 | crate | 表（ensure_table） |
|------|-------|--------------------|
| auth | `auth` | auth_users / auth_refresh_tokens |
| tokens | `catalog::tokens` | api_tokens |
| channels | `catalog::channels` | api_channels |
| groups | `catalog::groups` | api_groups |
| logs | `observe::logs` | usage_logs |
| monitor | `observe::monitor` | monitor_history |

共享一个 `Arc<AuthService>`（JWT 校验/ bearer 解析全仓一处）。

## 集成

```rust
// apps/api/src/main.rs
let admin = admin_api_router::router(pool.clone()).await?;
let app = gateway.router().merge(admin);
```

`router()` 返回 `Result`：DDL 失败或 `FERRITE_JWT_SECRET` 缺失时由调用方决定日志/退出。

## 注意

- 每次调用都会跑全部 ensure_table（`IF NOT EXISTS` 幂等，并发首启安全 —
  raw_sql 简单查询协议）；**调用一次即可**，不要在测试/热路径反复调。
- 新增子域时：子域 crate 提供 `ensure_table` + `router(state)`，在 `lib.rs`
  增加一段接线并复用 `auth_svc`。
