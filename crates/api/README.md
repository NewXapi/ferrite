# api — 后端服务

`apps/api`（bin `ferrite`）组装的全部后端 crate。分两个子域：

- **admin 域**（管理面，经 `admin-router` 聚合挂管理员鉴权守卫）
- **tavern 域**（酒馆面，全部坐 `tavern-storage`）

## crate 清单

| crate | 职责 | tests/ |
|---|---|---|
| `auth` | 登录/注册/refresh/logout/self + OIDC 身份解析（Argon2id + JWT 刷新轮换） | error_status / integration / unit |
| `auth-oidc` | OIDC 协议层：provider 注册表、state 一次性存储、授权码+PKCE 流程 | — |
| `admin-router` | admin 域路由聚合点，挂管理员鉴权守卫 | — |
| `admin-catalog` | 渠道 / 模型 / 分组 / Token（平表直连） | channels_groups / model_pricing_wire / models_channels / tokens |
| `admin-billing` | 钱包、兑换码、订单、订阅、联盟奖励、币种 | 10 个文件 |
| `admin-observe` | 用量日志、统计聚合、渠道探活监控 | gateway_health / logs |
| `admin-ops` | 系统信息诊断、网关热更 options | system_info |
| `admin-proxy` | 出口代理节点管理（`[[proxy_nodes]]` 的 DB 化写侧） | proxy_nodes / report / subscription |
| `db-bootstrap` | sqlx 迁移的单一权威入口 | — |
| `tavern-storage` | 酒馆数据根目录与文件读写底座 | paths |
| `tavern-auth` | 请求身份 → 用户目录的唯一入口 | identity |
| `tavern-characters` | 角色卡 CRUD（含 PNG 处理） | cards / png_chunk |
| `tavern-chats` | 聊天记录存取（JSONL 一行一条消息） | jsonl |
| `tavern-generate` | 生成请求转发 + SSE 透传 + 中止 | marker |
| `tavern-presets` | 单用户预设 JSON 文件 | presets |
| `tavern-secrets` | 用户自己的上游密钥 | masking |
| `tavern-settings` | 用户设置读存 | roundtrip |

## 依赖与越界边（实测）

- admin-* 之间经 `auth` / `db-bootstrap` / `admin-catalog` 互联
- tavern-* 全部坐 `tavern-storage`
- **跨域越界边**（名义规则是「只走 contract」，这些是实测存在的）：
  - `admin-observe → gateway/dispatch`
  - `admin-proxy → gateway/proxy`
  - `admin-router → gateway/{dispatch,proxy}`
  - `tavern-generate → harness/prompt`
- `Cargo.toml` 改名依赖：`client` = `admin-client`、`page-auth` / `page-account` /
  `page-overview` / `page-admin` / `page-users` = `admin-page-*`

## 硬约束

- 测试放同层 `tests/`，**不得**在 `src/` 里写 `#[cfg(test)]`
- 错误语义：`AuthError::Jwt(_)` 必须 401（前端 401-refresh 链路依赖），
  `Db/Crypto/Internal` 才是 500

## 验收

```bash
cargo check -p auth -p auth-oidc -p admin-router -p admin-catalog -p admin-billing \
  -p admin-observe -p admin-ops -p admin-proxy -p db-bootstrap \
  -p tavern-storage -p tavern-auth -p tavern-characters -p tavern-chats \
  -p tavern-generate -p tavern-presets -p tavern-secrets -p tavern-settings
```
