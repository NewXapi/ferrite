# Ferrite

单机 API 聚合平台：**OpenAI 兼容中转网关 + 管理后台 + 酒馆前端**。
Rust workspace，后端 Axum，前端 Dioxus（编译到 `wasm32-unknown-unknown`）。

一个 `ferrite` 进程同时承载 admin-api、tavern-api 与网关数据面；
前端与管理后台分离部署，经同一后端的 `/api/*` 与 `/v1/*` 通信。

## 当前进度

- **网关数据面**：OpenAI 兼容转发全链路可用——准入闸门（认证/配额/分组）→
  渠道调度（健康度 + 加权随机 + failover）→ 上游转发（SSE 流式透传）→
  计量结算（写 `usage_logs` 平表）。出站代理支持 VMess / VLESS / Shadowsocks /
  Trojan / WebSocket 传输与 uTLS 指纹，节点池已接入数据面并支持热更新。
- **admin-api**：账号中心（Argon2id + JWT + 刷新轮换）、渠道/模型/分组/Token/
  路由单元 CRUD、兑换码生成与核销、用量日志与统计聚合（日/时/月桶、Top 榜）、
  渠道探活监控、系统信息诊断、网关热更（`POST /api/gateway/reload`）。
- **管理后台（admin-web）**：总览（实时趋势/Top 榜/渠道健康/统计）、账户
  （密钥管理/用量日志/资料与会话）、管理（用户/分组/渠道/网络拓扑/别名/
  兑换码/系统诊断）**全部接入真实 API**。登录态持久化 + 401 静默刷新。
- **酒馆（tavern-web）**：角色卡/聊天/预设/ lorebook 页面与聊天链路
  （经 harness runtime 接上游模型）。

## 路由

```text
管理后台    ──→  /admin/*     管理面（admin-web 静态资源 + /api/*）
酒馆前端    ──→  /tavern/*    酒馆面
客户端      ──→  /v1/*        OpenAI 兼容中转（网关数据面）
```

## 快速开始

```sh
cp config/config.toml.example config/config.toml   # 本地配置（不入库）

cargo run -p api                        # 后端单体（bin 名 ferrite）
cargo run -p gateway                    # 网关数据面独立进程（可选）

cd apps/admin-web  && bun install && dx serve --port 8182   # 管理后台
cd apps/tavern-web && bun install && dx serve               # 酒馆前端
bun run css                             # Tailwind watch
```

数据库使用本地 PostgreSQL（连接串在 `config/config.toml`），表结构由各
服务启动时自动创建。

## 目录结构

```text
apps/                    # 进程入口，只做配置、状态与路由组装
├── api/                 # 后端单体：admin-api + tavern-api + harness runtime
├── gateway/             # 网关数据面进程
├── admin-web/           # 管理后台前端
└── tavern-web/          # 酒馆前端

crates/
├── contract/            # 跨端共享 DTO 与协议错误，前后端唯一共享点
├── api/                 # 后端服务（详见 crates/api/admin.md、tavern.md）
│   ├── auth/            #   平台账号中心（Argon2id + JWT）
│   ├── admin-*/         #   catalog / billing / observe / ops / proxy / router
│   └── tavern-*/        #   storage / auth / characters / chats / settings /
│                        #   presets / secrets / generate
├── web/                 # 前端组件与页面（详见 crates/web/admin.md、tavern.md）
│   ├── ui-components/   #   跨端通用组件
│   ├── admin-client/    #   admin 侧 API client / session
│   ├── admin-page-*/    #   auth / overview / account / users / admin
│   ├── tavern-client/   #   tavern 侧 API client / state
│   └── tavern-page-*/   #   home / characters / chat / personas / lorebook / settings
├── gateway/             # 网关数据面（详见 crates/gateway/README.md）
│   ├── pipeline/        #   Stage trait 与请求上下文
│   ├── gate/            #   准入与配额闸门
│   ├── dispatch/        #   渠道选择：健康度 + 加权随机 + failover
│   ├── forward/         #   上游转发与 SSE 流式透传
│   ├── protocol-bridge/ #   厂商协议适配（OpenAI / Claude / Gemini）
│   ├── metering/        #   token 计量与结算
│   ├── proxy/           #   出站代理拨号（VMess/VLESS/SS/Trojan…）
│   └── security/        #   内容安全与敏感词
└── harness/             # Agent 运行时（详见 crates/harness/README.md）
    ├── core/            #   run 状态机
    ├── prompt/          #   prompt 组装与截断
    ├── tools/           #   工具调用
    ├── runtime/         #   事件循环与持久化
    └── tokenizer/       #   token 计数

tests/                   # 跨 crate 端到端集成测试
scripts/                 # CI 动态选包等工程脚本
docs/                    # 专题文档（数据库迁移、代理节点等）
```

## 域文档

| 域 | 文档 |
|---|---|
| 后端 admin 域 | [`crates/api/admin.md`](crates/api/admin.md) |
| 后端 tavern 域 | [`crates/api/tavern.md`](crates/api/tavern.md) |
| 管理后台前端 | [`crates/web/admin.md`](crates/web/admin.md) |
| 酒馆前端 | [`crates/web/tavern.md`](crates/web/tavern.md) |
| 网关数据面 | [`crates/gateway/README.md`](crates/gateway/README.md) |
| Agent 运行时 | [`crates/harness/README.md`](crates/harness/README.md) |
| 共享契约 | [`crates/contract/README.md`](crates/contract/README.md) |

## 开发约定

开发流程、门禁（gate）、测试分层、CI 动态选包等工程约定见
[`AGENTS.md`](AGENTS.md)。要点：

- 域间禁止直接依赖：跨域引用只允许经 `crates/contract`。
- `crates/web/*`、`apps/{admin-web,tavern-web}`、`harness/{core,prompt,tools}`
  必须能编译到 `wasm32-unknown-unknown`。
- 测试放同层 `tests/`，不在 `src/` 里写 `#[cfg(test)]`；本地只做类型检查，
  全量测试交给 CI。
- CI：PR 只跑受改动影响的包（`scripts/ci-affected.sh` 动态选包），
  push 到 main 跑全量兜底。
