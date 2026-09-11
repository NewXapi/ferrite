# `crates/api` — 后端服务大域

全部后端服务平铺大容器，包含 `auth`（通用账号中心）、`admin-*`（管理服务）、`tavern-*`（酒馆服务）。

## 子域划分

- **admin 域**（见 [admin.md](admin.md)）：`auth` / `admin-catalog` / `admin-billing` / `admin-observe` / `admin-ops` / `admin-proxy` / `admin-router` / `db-bootstrap`
- **tavern 域**（见 [tavern.md](tavern.md)）：`tavern-storage` / `tavern-auth` / `tavern-characters` / `tavern-chats` / `tavern-settings` / `tavern-secrets` / `tavern-presets` / `tavern-generate`

## 架构约束

- 跨域共享只有 `crates/contract`（共享 API 契约）；需要新 DTO 先声明变更，由一个会话统一修改
- 域间禁止直接私有依赖；跨端数据交互必须基于 `contract` DTO
- 每个功能 crate 是独立 Cargo Library Crate，各自拥有独立的 `Cargo.toml`、`src/lib.rs` 与 `tests/`
- `lib.rs` 尽量只放共用结构体与 trait，实现在各子文件里

## 装配

- `apps/api` 统一组装 `crates/api/*` 与 `crates/gateway/*`、`crates/harness/runtime`
- `admin-router` 是 admin 域路由聚合点，挂管理员鉴权守卫
