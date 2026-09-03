# `apps`

## 目录

```text
apps/
├── api/
├── web/
└── tavern-web/
```

## 要实现

- `api` 读取配置、初始化 PostgreSQL、酒馆数据目录和共享状态。
- `api` 挂载 `/v1/*` 网关路由、`/admin/*` 管理路由和 `/tavern/*` 酒馆路由。
- `web` 组装 admin-web 页面和路由。
- `tavern-web` 组装 tavern-web 页面、harness UI 和路由。
