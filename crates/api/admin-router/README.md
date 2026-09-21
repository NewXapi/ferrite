# admin-router

admin 域路由聚合点，挂管理员鉴权守卫。

## 职责

把各 admin-* crate 的子路由聚合成一棵 `/api/admin/*` 树，并在挂载点统一套管理员
鉴权（`auth` 导出的 `bearer_user()` + `ADMIN_ROLE_THRESHOLD`）。自身不含业务逻辑。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/lib.rs` | 路由聚合与守卫挂载 |

## 依赖

`{auth, admin-catalog, admin-billing, admin-observe, admin-ops, admin-proxy}` +
越界边 `gateway/{dispatch,proxy}`。

## 验收

```bash
cargo check -p admin-router
```
