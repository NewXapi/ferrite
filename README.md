# Ferrite

**自托管单机 AI 网关：OpenAI 兼容中转 + 管理后台 + 酒馆前端，一个 Rust 二进制。**

Ferrite 把多家上游模型服务（OpenAI / Claude / Gemini 及兼容端点）聚合到一个
自托管网关后面：客户端只对接 `/v1/*` OpenAI 兼容接口，网关负责准入
（认证 / 配额 / 分组）、渠道调度（健康度 + 权重 + failover）、上游转发
（SSE 流式透传、多协议编解码）与计量结算；`/admin/*` 管理后台覆盖渠道 /
模型 / 用户 / 用量 / 兑换码的运营视图；`/tavern/*` 酒馆前端提供角色卡聊天，
经内置 Agent 运行时（harness）接上游模型。

**适合场景**：个人或小团队自持多把上游 key，需要统一入口、分组授权、用量审计
与计费结算，或不方便把上游密钥散落到各个客户端。全部组件编译进单个
`ferrite` 进程（前端为 wasm），数据落在自己的 PostgreSQL 里。

**技术栈**：Rust workspace——后端 Axum，前端 Dioxus 编译到
`wasm32-unknown-unknown`。一个 `ferrite` 进程同时承载 admin-api、tavern-api
与网关数据面；前端与管理后台经同一后端的 `/api/*`、`/tavern/*`、`/v1/*` 通信。
区别于同类 Go 系网关：Rust 单二进制、wasm 前端内嵌、内置 Agent 运行时与
酒馆前端。

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
服务启动时自动创建。dev 环境（共享后端 3211 / dx / 标注栈）走 `justfile`
配方，web 双车道流程见 `.agent/rules/web-lanes.md`。

## 目录结构

```text
apps/                    # 进程入口：只做配置、状态与路由组装
├── api/                 #   后端单体 src/ tests/
├── gateway/             #   网关数据面进程 src/ tests/
├── admin-web/           #   管理后台 wasm：src/ tests/ assets/ scripts/(ainotation-bridge.mjs)
│                        #     Dioxus.toml ainotation-entry.ts
├── leptos-web/          #   Leptos SSR 试用应用（SSR-only，不出 wasm）
└── tavern-web/          #   酒馆前端 wasm：src/ assets/ Dioxus.toml（无 tests/）

crates/
├── contract/            # 跨域契约层：src/{api,records}/ tests/（零内部依赖）
├── gateway/             # 数据面热路径（全部 crate 带 tests/）
│   ├── pipeline/        #   ctx/stage/pipeline/router
│   ├── gate/            #   auth/state/quota/ratelimit/model/graylist/concurrency + snapshot/
│   ├── dispatch/        #   candidate/health/selector/retry/ratelimit/stage
│   ├── forward/         #   stage/stream/pipeline/egress/adapter(_egress)/stream_resilience
│   ├── protocol-bridge/ # openai/claude/gemini/ir/sse/format_codec/stage
│   ├── proxy/           #   manager/node/pool/probe/ssrf/sharelink/adapter
│   ├── security/        #   scan/wordlist（零消费方）
│   └── metering/        #   estimate/pricing/ledger/scanner/settle/sink
├── api/                 # 后端服务（admin-router、db-bootstrap 无 tests/）
│   ├── auth/            #   service/jwt/password/routes/error
│   ├── admin-catalog/   #   channels/models/groups/tokens
│   ├── admin-billing/   #   wallet/redeem/topup(_epay)/subscriptions/affiliate/currency
│   ├── admin-observe/   #   logs/monitor/gateway_health
│   ├── admin-ops/       #   system_info/options
│   ├── admin-proxy/     #   subscription
│   ├── admin-router/    #   lib.rs（路由聚合）
│   └── tavern-*/        #   storage/auth/characters/chats/generate/presets/secrets/settings
│                        #   （tavern-* 多为 lib.rs + http.rs 两件套）
├── harness/             # Agent 运行时（vectors 未接入 workspace members）
│   ├── core/            #   run/status/event/plan/profile/storage
│   ├── prompt/          #   render/truncate/variables/world_info/reasoning
│   ├── tools/           #   spec/format/gate/result/adapter
│   ├── runtime/         #   loop_engine/turn/tool_exec/persistence/provider
│   ├── tokenizer/       #   engine/registry（零消费方）
│   └── vectors/         #   chunk/hash/index/recall
└── web/                 # 前端
    ├── ui-components/   #   src/components/ 17 个组件目录 + mod.rs（含 admin_card/ 卡牌族）
    │                    #   + card/bubble/dialog/form/feedback/segmented/session/i18n 等
    ├── admin-client/    #   lib/setup_client/manage_auth_token/wire
    ├── admin-page-auth/         # api/form/state/view
    ├── admin-page-overview/     # api/shared + tab-page-{overview,models,leaderboard}/
    ├── admin-page-account/      # api/usage_support + tab-page-{keys,usage-logs,rewards,sessions,settings}/
    ├── admin-page-admin/        # api/state/drawer_write + tab-page-{aliases,channels,currency,
    │                           #   entities,gateway,groups,network,redemptions,subscriptions,system}/
    ├── admin-page-users/        # api/data + tab-page-users/
    ├── tavern-client/           # lib.rs 单文件
    ├── tavern-state/            # lib/dock
    └── tavern-page-*/           # home/characters/chat/personas/lorebook/settings
                                # （chat 含 dock/dock_panels/layout；characters 内嵌 personas+lorebook）

tests/                   # 顶层跨 crate 集成测试：auth_flow / gateway_e2e / billing_lifecycle / admin_gateway_flow
e2e/                     # 管理页 Playwright E2E（本地跑，不在 CI）
db/                      # migrations/（sqlx 迁移）+ dev/（种子生成器）+ optional/
config/                  # config.toml（gitignore）+ config.toml.example
scripts/                 # ci-affected.sh / dev-backend.sh / wt-clean.sh
benches/                 # 性能基准（data/ results/ scripts/，非 cargo bench）
```

## crate 职责（当前 workspace 成员）

### contract — 全 workspace 逻辑契约层

- `contract`：DTO / records / 错误码 / schema 的唯一事实来源。**不依赖**
  tokio/sqlx/axum/reqwest/dioxus，必须过 `wasm32` check。SQL DDL 与存储
  编码不在本 crate。

### gateway — 网关数据面（热路径）

编排顺序：`pipeline`（上下文 + Stage 链）→ `gate`（准入）→ `dispatch`
（选路）→ `forward`（转发）→ `metering`（计量旁路）；`protocol-bridge` 做
编解码适配，`proxy` 管出口，`security` 做内容扫描。

- `gateway-pipeline`：请求上下文、Stage 接口、链执行器和 HTTP 路由（编排核心）
- `gateway-gate`：准入过滤——auth / state / quota / ratelimit / model / graylist / concurrency
- `dispatch`：候选渠道、健康状态、权重选择与失败重试
- `forward`：上游 URL/头/请求体/响应体/SSE 流转发与协议适配
- `gateway-protocol-bridge`：pipeline 上下文 ↔ protocol codec 输入输出适配
- `gateway-proxy`：出口节点解析、按 channel_key 租 HTTP/SOCKS5 Client、SSRF 防护
- `gateway-security`：词库、输入替换、跨 chunk 扫描与审核结果（**当前零消费方**：apps/api 与 apps/gateway 均未依赖）
- `metering`：预扣额度、token 估算、定价与结算

### api — 后端服务

admin 域（管理面，经 `admin-router` 聚合挂管理员鉴权守卫）：

- `auth`：登录/注册/refresh/logout/self（Argon2id + JWT 刷新轮换）
- `admin-catalog`：渠道 / 模型 / 分组 / Token（平表直连）
- `admin-billing`：商业化域——钱包、兑换码、订单、订阅、联盟奖励、币种
- `admin-observe`：观测聚合——用量日志、统计聚合、渠道探活监控
- `admin-ops`：运维域——系统信息诊断、网关热更
- `admin-proxy`：出口代理节点管理（`[[proxy_nodes]]` 的 DB 化写侧）
- `admin-router`：admin 域路由聚合点，挂管理员鉴权守卫
- `db-bootstrap`：sqlx 迁移的单一权威入口

tavern 域（酒馆面）：

- `tavern-storage`：酒馆数据根目录与文件读写底座
- `tavern-auth`：请求身份到用户目录的唯一入口
- `tavern-characters`：角色卡 CRUD（含 PNG 处理）
- `tavern-chats`：聊天记录存取（JSONL 一行一条消息）
- `tavern-generate`：生成请求转发 + SSE 透传 + 中止（直连 harness-prompt）
- `tavern-presets`：单用户预设 JSON 文件
- `tavern-secrets`：用户自己的上游密钥
- `tavern-settings`：用户设置读存

### harness — Agent 运行时

- `harness-core`：Run/Step 状态机、取消与序列化（零 runtime 依赖）
- `harness-prompt`：系统提示、角色资料、历史、变量展开与上下文裁剪
- `harness-tools`：工具契约（ToolSpec/ToolCall/ToolResult），只声明不执行
- `harness-runtime`：模型-工具循环、审批、持久化、步骤事件流（仅后端）
- `harness-tokenizer`：真实 tokenizer（**当前零消费方**：`harness-runtime/src/bias.rs` 的 encode 由调用方注入，无 crate 依赖本 crate）
- `harness-vectors`：向量检索记忆（**未接入 workspace members 且零消费方**）

### web — 前端

管理端（消费 `/api/*`）：

- `admin-client`：管理 API 客户端——Bearer 注入、`Envelope<T>` 解码、401 回调 refresher
- `admin-page-auth`：登录/注册认证页（state context + 公开入口）
- `admin-page-overview`：总览/模型/排行榜三个 tab（`tab-page-*/` 目录化）
- `admin-page-account`：个人中心——密钥、用量日志、会话、奖励、设置（tab 目录化）
- `admin-page-admin`：管理操作——渠道/分组/别名/兑换/网络/系统等 10 个 tab（`tab-page-*/` 目录化）
- `admin-page-users`：用户管理面板（数据经 api 层取用）

酒馆端（消费 `/tavern/*`，含 SSE 流式）：

- `tavern-client`：`/tavern/*` 请求 + `generate` SSE 分帧解析
- `tavern-state`：角色/聊天/消息全局状态 + 生成中状态 + dock 布局状态
- `tavern-page-home`：品牌落地页
- `tavern-page-characters`：角色/剧本库、创作中心（内嵌 personas/lorebook 面板）
- `tavern-page-chat`：聊天与流式生成互动界面（dock 布局）
- `tavern-page-personas`：用户人格管理
- `tavern-page-lorebook`：世界书管理
- `tavern-page-settings`：连接与采样设置

跨端共享：

- `ui-components`：跨端通用组件层，**只依赖 contract，不依赖任何 client/page**。
  `src/components/` 17 个组件目录（avatar/badge/button/card/dropdown_menu/input/
  layout/rank_board/select/sheet/showcase/sidebar/skeleton/stat_card/switch/toast 等），
  其中 `admin_card/` 是管理区卡牌族：`AdminCard` 四态面板 + `CardShell` 语义壳 +
  `Pager` 分页 + `PriceMode` + aliases/channels/users 实体卡 + `editable` 行内编辑。

### apps — 进程入口（只做配置、状态与路由组装）

- `apps/api`（bin `ferrite`）：后端单体，组装全部 `crates/api/*` + `crates/gateway/*`（security 除外）+ contract
- `apps/gateway`：网关数据面进程，只把各 gateway crate 拼成 pipeline
- `apps/admin-web`：管理后台 wasm 应用（挂 tailwind + dx-components-theme，`init_auth()` 注册 401 静默刷新）
- `apps/leptos-web`：Leptos 0.8 SSR 试用应用（axum 直出 HTML，**SSR-only，不在 wasm32 硬约束内**），用来量 leptos dev 期 CPU/内存，与 dx serve 对比
- `apps/tavern-web`：酒馆前端 wasm 应用（只挂 tailwind，**不挂** dx-components-theme）

## 依赖与组装规则

以下边全部实测自各 `Cargo.toml` 的 path 声明（2026-09-21）：

- **组装只发生在 `apps/*`**；page crate 互相不感知（唯一例外：`tavern-page-characters`
  内嵌 personas/lorebook 面板）。
- **契约层**：`ui-components` / `admin-client` / `tavern-client` / `metering` /
  `pipeline` / `gate` / `protocol-bridge` / `auth` 均只依赖 `contract`（+ 域内必要件）。
- **域内依赖**：gateway 域 `dispatch→{pipeline,gate,forward,protocol-bridge}`、
  `forward→{pipeline,dispatch,protocol-bridge,proxy,metering}`；harness 域
  `runtime→{core,prompt,tools}`、`prompt→core`、`tools→core`；api 域 admin-* 之间
  经 `auth`/`db-bootstrap`/`admin-catalog` 互联，tavern-* 全部坐 `tavern-storage`。
- **跨域依赖（名义规则是“只走 contract”，实测存在的越界边，改动时注意）**：
  - api → gateway：`admin-observe → gateway/dispatch`、`admin-proxy → gateway/proxy`、
    `admin-router → gateway/{dispatch,proxy}`
  - api → harness：`tavern-generate → harness/prompt`
  - web → harness：`tavern-state → harness/prompt`（前端唯一越界边）
- Cargo.toml 里的**改名依赖**（读代码时按 key 认依赖）：`ui` = `ui-components`、
  `client` = `admin-client`、`tavern_client` / `tavern_state`、`page-auth` /
  `page-account` / `page-overview` / `page-admin` / `page-users` = `admin-page-*`。
- `lib.rs` 尽量只放共用结构体与 trait，实现在各子文件里。

## 契约约定（contract，改 DTO 前必读）

- 所有 DTO 一律 `#[serde(rename_all = "camelCase")]`；列表端点统一包
  `{"items": [...], "total": N}`；auth 系（login/register/refresh/self）返回裸 JSON。
- **计费单位 `500000 = ¥1`**（quota / used_quota / 兑换码面额同量纲）。
- 状态词表：token/channel `1=启用 2=停用`；兑换码 `1=未用 2=停用 3=已核销`；
  `logType 2=消费`。
- 错误语义：`AuthError::Jwt(_)` 必须 401（前端 401-refresh 链路依赖），
  `Db/Crypto/Internal` 才是 500。
- 分页参数：后端 `LogQuery` 是 `page/size`（`size` 服务端 clamp 100）；
  `start/end` 均为 RFC3339 UTC。
- 已锁定的字段错位（有回归测试，改动前先读测试）：`UserDto.role` 是整数
  （`1/10/100`，语义化走 `role_label()`）；`UserDto.requestCount` 后端不下发
  （`Option<u64>`）；`TokenDto` 列表字段直名 `key_preview`；创建 token 响应嵌套
  `{plaintext, token}`（明文只出现一次）；`UsageLogDto` 无 success/cost 列
  （前端从 `logType==2` 派生、`quota/500000` 换算费用）；兑换码 `DELETE = 停用`。
- DTO 唯一事实来源是各 admin-api crate 的 `*View` 结构体；改后端序列化必须同步
  本 crate，**先用真实抓包 JSON 写回归测试再改实现**。
- 判断"某修复是否已在 main"必须 `git merge-base --is-ancestor`，
  `git log --all --grep` 会误报（可能命中的是其他分支）。

## 硬约束

- `crates/harness/{core,prompt,tools}`、`crates/web/*`、`apps/{admin-web,tavern-web}`
  必须支持 `wasm32-unknown-unknown`（`cargo check --target wasm32-unknown-unknown -p <crate>`）。
- 测试放同层 `tests/`，不在 `src/` 里写 `#[cfg(test)]`；本地只做类型检查，
  全量测试交给 CI（PR 按 `git diff` 动态选包，`scripts/ci-affected.sh`）。
- 公共 API 写 `///` rust doc，模块头 `//!` 说职责。
- 新增或移动功能 crate 时，更新根 `Cargo.toml` 的 `workspace.members`。

## 工程约定

开发流程、门禁（gate）、测试分层等约定见 [`AGENTS.md`](AGENTS.md)；文档导航见
`.agent/README.md`。代码结构调查用 code-review-graph（`cg`）。
