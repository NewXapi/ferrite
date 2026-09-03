# `crates/admin-api/router`

聚合 admin-api 子域的 axum Router，供 `apps/api::main` 一次性挂载。

```rust
// apps/api/src/main.rs
let admin = admin_api_router::router(pool.clone()).await;
let app = gateway.router().merge(admin);
```

当前包含 `auth` (login/register/refresh/logout/self)。后续 `catalog/users`、
`channels` 等子域再加进 `lib.rs::router`。
