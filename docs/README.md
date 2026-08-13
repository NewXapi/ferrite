# Rust 重写：文档索引

> 日期：2026-08-13
> 调查工具：codegraph（1862 文件 / 31146 点 / 78913 边）+ 逐条 file:line 复核
> 全部引用（299 处 file:line）、数量指标（47 项）、符号数（45 项）均已程序化校验

## 文档结构

**08-09 是新应用的设计**（要建什么）。01-07 是现有系统的调查，只在拆分时作参考。

每份应用文档的内容都是同一套：**内部功能 + 内部数据流动 + 目录结构**。
应用之间的数据流动单独成篇。

### 新应用设计

- `08-mvp.md` — **MVP**：`omp-relay` 单二进制 CLI。零外部依赖，配置从 TOML 读
- `09-roadmap.md` — **路线图**：① 单应用能力 → ② 应用交互 → ③ 集群增强

### 现有系统调查（参考）

- `01-gateway.md` — 推理网关
- `02-control.md` — 管理控制面
- `03-worker.md` — 后台任务
- `04-egress.md` — 出网代理
- `05-nginx-web.md` — 入口与前端静态资源
- `06-datastores.md` — postgres / log-db / redis
- `07-inter-app-dataflow.md` — 应用之间的数据流动（F1–F17）

## 应用总览

- **gateway** — Rust / axum / reqwest / tokio，监听 `:8080`，66 条路由，N 副本
- **control** — Rust / axum / sqlx，监听 `:8081`，266 条路由，2 副本（1 个持迁移锁）
- **worker** — Rust / tokio，无业务 HTTP 端口，14 类任务，N 副本
- **egress** — 官方 sing-box 镜像，监听 `:1080` SOCKS5，1/节点
- **nginx** — `nginx:alpine`，`:80` / `:443`，按 7 类路径前缀分发
- **web** — 静态文件卷，由 nginx 托管，不是进程
- **postgres** — 主库 34 表
- **log-db** — 独立 DSN，1 张表 `Log`
- **redis** — 缓存 + 限流 + 新增失效总线

## 今天的起点

- 单一 Go 二进制 `app/api`，133762 行
- 独立 Go 模块 `modules/relaykit`，15316 行非测试代码，零 `app/api` 依赖
- 332 条路由（`app/api/router/*.go`，含 `channel-router.go:31-36` 的 39 条表驱动注册）

## 三个决定拆分难度的事实

**① 适配器层已经是纯的。**
`relay/channel/**` 全量扫描 `model.DB` / `model.Get*` / `model.Cache*` / `model.Record*` 等模式，
**DB 调用数 = 0**。40 个 provider 目录、34 个 `Adaptor` 实现、10 个 `TaskAdaptor` 实现，
全部只做请求/响应变换。整个 `relay/` 的 DB 访问只在 2 个文件：
`relay/mjproxy_handler.go`（19 处）、`relay/relay_task.go`（6 处）。
→ `protocol` 与 `adaptor` 两个 crate 可以零 IO，排在迁移最前面。

**② gateway 完全不需要会话。**
对 `relay/`、`controller/relay.go`、`middleware/distributor.go` 扫
`UserSession` / `ValidateLoginSession` / `ParseAccessToken`，**零命中**。
gateway 只需 `middleware/auth.go:408` `ValidateUserToken`。
→ 整个 identity 模块不进 gateway，这是免费得到的干净边界。

**③ 跨实例一致性已经全靠 DB + Redis，没有 pub/sub。**
`grep -rn 'Subscribe|Publish'` 只命中 `model.PublishUserAuthCache`
（`model/user_auth_cache.go:229-235`），那是误导性命名，实际是
`GetUserById` + `updateUserCache`，不是 pub/sub。
→ 拆进程不需要发明分布式协议，只需给渠道/定价缓存补一条失效通知（见 `07-inter-app-dataflow.md` F8）。

## 阻碍拆分的 5 份状态

- 渠道路由表 — `model/channel_cache.go:19` `group2model2channels` / `channelsIDM`，进程内 map
- 定价倍率表 — `setting/ratio_setting/model_ratio.go:324-326`，进程内 `RWMap`
- sing-box 拨号器 — `service/singbox_dialer.go:337` `globalSingBoxDialer`，进程内跑真实 `box.Box`
- 配额批量累加 — `model/utils.go:23` `batchUpdateStores`，5 类进程内 `map[int]int`
- 用量小时聚合 — `model/usedata.go` `CacheQuotaData`，进程内 map

前两份靠 F8 失效通知解决，第三份靠拆 egress 解决，后两份必须删掉或归单点。
