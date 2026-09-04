# `catalog`

## 文件

- `src/lib.rs` — 模块地图与原 Catalog trait（store-trait 设计态骨架，sync/outbox
  推迟；channels/groups/tokens 已由平表直连 sqlx 的实现替代，骨架保留仅作文档参考）。
- `src/channels.rs` — 渠道 CRUD/search/启停 + **探活**（`test_channel`/`test_all`，
  reqwest 真调 chat/completions，结果落 observe::monitor）+ axum 路由（9 端点）。
- `src/groups.rs` — 分组 CRUD、倍率、白名单，default 组保护与引用检查 + 路由（4 端点）。
- `src/tokens.rs` — API Key 创建（明文一次性/sha256 入库）、CRUD/search、
  `regenerate_key`（重取=重新生成）+ 路由（6 端点）。
- `src/routes.rs` — 模型到渠道 RouteUnit 映射（**骨架未实现**，等 gateway 接线）。
- `src/users.rs` — 管理用户角色、状态和配额（**骨架未实现**，auth::service::manage_user 已覆盖）。

## 表（loose，无 FK）

- `api_tokens` — key/user_key/name/key_hash(sha256)/key_preview/group_id/quota/…
- `api_channels` — key/name/channel_type/base_url/keys(JSONB)/models(JSONB)/group_name/priority/weight/status/test_model
- `api_groups` — key/name/ratio/model_whitelist(JSONB)/…（default 组 seed 保护）

## 路由

| 方法 | 路径 | 鉴权 |
|------|------|------|
| POST/GET | `/api/channel`, `/api/channel/search` | admin |
| GET/PUT/DELETE | `/api/channel/{key}` | admin |
| POST | `/api/channel/{key}/status`, `/api/channel/{key}/test`, `/api/channel/test` | admin |
| POST/GET | `/api/token`, `/api/token/search` | user（列表 all=true 需 admin）|
| PUT/DELETE | `/api/token/{key}` | owner 或 admin |
| POST | `/api/token/{key}/key` | owner 或 admin |
| GET/POST | `/api/group` | admin |
| PUT/DELETE | `/api/group/{key}` | admin |

## 验证

```sh
DATABASE_URL=postgres://<user>:<pass>@127.0.0.1:5433/<db> \
    cpulimit -l 70 -i -- cargo test -p catalog --tests -- --include-ignored
cargo clippy -p catalog --all-targets
```
