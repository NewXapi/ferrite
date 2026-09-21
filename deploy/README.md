# Ferrite 部署

一个仓库两种形态，形态由 `apps/api` 的 cargo feature 决定，**在构建时选**，
不在运行时选。docker 镜像按形态打不同 tag。

| | 全功能 | 个人 |
|---|---|---|
| 镜像 | `ghcr.io/newxapi/ferrite-api:latest` | `ghcr.io/newxapi/ferrite-api:personal` |
| feature | `default`（tavern + billing） | `--no-default-features` |
| 前端 | admin-web（:8080）+ tavern-web（:8081） | 只有 admin-web |
| 后端路由 | 网关域 + 计费域 + tavern 域 | 网关域（计费/tavern 不编译进去） |

## 快速跑（compose，无需本地编译）

```sh
cd deploy
cp .env.example .env
$EDITOR .env            # 至少改 FERRITE_JWT_SECRET

# 全功能形态
docker compose up -d
open http://localhost:8080    # 后台管理
open http://localhost:8081    # 酒馆

# 个人形态
docker compose -f compose.yml -f compose.personal.yml up -d
open http://localhost:8080
```

后端 API 直连在 `:3000`（compose 发布端口）。web 容器内嵌 caddy，
把 `/api`、`/tavern`、`/v1` 反代到 api 容器（web 端用相对路径，同源契约）。

个人形态的健全性检查（形态差异本身可观测）：

```sh
curl -o /dev/null -w '%{http_code}\n' http://localhost:8080/api/user/wallet   # 404：计费域不存在
curl -o /dev/null -w '%{http_code}\n' http://localhost:8080/api/channel       # 401：网关域存在
```

## 手动起单镜像

```sh
docker build -f deploy/api/Dockerfile .                              # 全功能
docker build -f deploy/api/Dockerfile --build-arg FEATURES=--no-default-features .   # 个人
docker build -f deploy/admin-web/Dockerfile .
docker build -f deploy/tavern-web/Dockerfile .
```

api 镜像运行时必须提供 `/app/config/config.toml` 与 `FERRITE_JWT_SECRET`
（模板见仓库 `config/config.toml.example`；compose 形态见 `deploy/config/config.toml`）。
web 镜像的 caddy 上游默认是 compose 服务名 `api`，脱离 compose 运行需改 Caddyfile。

## 发布（tag 触发）

版本口径见仓库 `VERSIONING.md` 与 `.agent/tasks/versioning.md`：
`api-v<major>.<minor>.<patch>`，major 是用户确认的 breaking、minor 是功能域数、
patch 是 `-- apps/api crates/gateway` 的 fix 提交数。切 tag 前重跑 pathspec 命令核对。

```sh
# 例（patch 计数以 main 实时重跑为准）
git tag api-v0.9.14 && git push origin api-v0.9.14
```

`.github/workflows/release.yml` 会构建四个镜像推到 GHCR，
每个同时打版本号 tag 与滚动 tag（`:latest` / `:personal`）。
compose 引用滚动 tag，要锁定发布版就把 compose 里的 `:latest` 改成版本号。

## 目录

```
deploy/
  compose.yml              全功能栈：postgres + api + admin-web + tavern-web
  compose.personal.yml     个人形态覆盖：api 换 :personal 镜像，禁用 tavern-web
  .env.example             FERRITE_JWT_SECRET（必填）、DB 口令（可选）
  config/config.toml       compose 用的 api 配置（database_url 指向 postgres 服务）
  api/Dockerfile           后端（ARG FEATURES 选形态）
  admin-web/Dockerfile     dx build → caddy 服务静态 + /api 反代（:8080）
  tavern-web/Dockerfile    dx build → caddy 服务静态 + /tavern、/v1 反代（:8081）
```
