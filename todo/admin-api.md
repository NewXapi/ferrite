# admin-api 开发路线图

> 单人单机版控制台 + 中转计费，对齐 new-api 核心功能。
> 远端 base: newxapi/main。工作目录: `.wt/<feature>/`（见 AGENTS.md）。
> 每次开工前 `git pull newxapi main` 刷新本文件进度。

## 全局背景与目标

**产品目标**：自建酒馆中转——控制台（admin-web）管理渠道/模型/token，网关（apps/api）对上游转发并按 token 计量扣除用户配额。

**核心闭环**：
```
用户注册 → 控制台创建 channel/token → 用户拿 sk- 放 SillyTavern → /v1/chat/completions
→ gateway 认证 + 路由 dispatch → 转发上游 → metering 计量 → 扣 auth_users.used_quota
→ 控制台/自查询额度、重启/兑换码充值
```

**约束**：单机部署、不做 ops 调度、不做分布式 sync、只用 PG 表。Relay 转发在 gateway 域（不进 admin-api）。

## 全局现状（按域）

| 域 | 状态 | 缺口 |
|---|---|---|
| auth 认证（注册/登录/refresh/logout/self/users） | ✅ 完整 | — |
| auth 会话（sessions） + 用户设置（settings） | ✅ 完整 | — |
| catalog 渠道（channels CRUD + 探活 + tag） | ✅ 完整 | fetch_models / update_balance 还是 stub |
| catalog 模型（models CRUD / missing） | ✅ 完整 | — |
| catalog 分组（groups） | ✅ 完整 | — |
| catalog token 生命周期（含补全） | ✅ 完整 | — |
| catalog 路由单元（route_units / routing） | ✅ 完成（PR #63） | 写侧 CRUD + 引用完整性 + 级联失效 |
| observe 日志（logs） + dashboard | ✅ 基本完整 | 缺 log-cleanup / audit / tokenlog 视图 |
| observe 监控（monitor 渠道可用率） | ✅ 完整 | — |
| billing 兑换码 / 订单 / 支付 | ✅ 兑换码完成 | orders/providers/subscriptions stub 已移除（git 历史保留设计），P2 按需恢复 |
| ops 调度 | 明确不做（单机版） | stub 已移除，options 平表实现替代 |
| 2FA / Org OAuth / Passkey | 明确不做 | — |
| Video / MJ / Relay+ 转发 | 明确不做（gateway 或 as-needed） | — |

## 已汇聚在 main 的新实现（含本次合并）

- `admin-catalog/src/models.rs`（7 端点：模型 CRUD / missing / search）
- `admin-catalog/src/tokens.rs` 扩展token 补全： auto-groups / batch / batch/keys / key get
- `admin-catalog/src/channels.rs` 追加 tag 批量 + 探活落 monitor_history + balance stub + fetch_models stub
- `auth/src/service.rs` + `ddl.rs`：auth_user_sessions / auth_user_settings 表 + 服务
- `auth/src/routes.rs`：/self/sessions /self/setting
- `admin-router` 接线： models router 挂载在 admin-api 聚合路由

## 下一步路线图（P0 → P3）

### P0 — 全局整合（前置，先做完再做其他）

**目标**：让控制台和网关共享同一数据源与逻辑，解决当前「控制台写 api_channels/api_tokens/api_models，但网关在 dispatch/routing 读 kv_store/旧 tokens」的断层问题。

| 项 | 作用 | 验收 |
|---|---|---|
| apps/api 数据源切到 admin-api 平表 | identity 读 api_tokens；dispatch 读 api_channels + route_units；删除 kv_store 旧路径 | /v1/* 和 /api/* 共享同库同表，控制台创建渠道后，发请求立即看到调用记录 |
| apps/api 挂载 admin-api 聚合 Router | /api/* 控制台路由与 /v1/* 网关路由在同一服务进程 | curl /api/token 能创建 token 并且 /v1/chat/completions 用这个 token 认证成功 |

**文件位置**（待调整）：apps/api/src/{gateway.rs,billing.rs,dispatch.rs}、crates/api/admin-*

不要先动业务功能。整合完成、以同一数据源 tracing 通过一次 chat/completions 全链路，控制台可见用量账单后，才允许往下对接新的子功能。

### P1 — 控制台可营销（对用户收钱）

| 项 | 状态 | 说明 |
|---|---|---|
| 兑换码充值 | ✅ 完成（PR #62） | billing_redemptions 平表 + CAS 核销 + auth_users.quota 入账；/api/redemption(admin) + /api/user/topup(用户) |
| 计量/计价计费 | ⏳ 依赖 P0 + gateway agent | metering settle 后扣 api_tokens.used_quota + auth_users.used_quota（契约见上） |
| 用户自助用量面板 | ✅ 已有 | /api/user/self(含 quota/used_quota) + /api/log/self + /log/self/stat |

### P2 — 控制台的可用性

- ✅ 系统选项（PR #63）：options 平表 + 类型化注册表（5 个首批选项）+ /api/option GET/PUT
- ✅ RouteUnit 写侧（PR #63）：route_units 平表 + CRUD + 引用完整性 + invalidate_by_channel 级联
- 删除账户：/api/user/self DELETE 已有
- 系统信息展示 /api/system-info：低优先，未做

### P3 — 快速收窄（暂不整合）

- 2FA（暂时不做——自用）
- OAuth（不做）
- Passkey（不做）
- RouteUnit 写侧完整（校验已有，CRUD 待定）— 待调度链完善后
- Video / MJ / relay helper（转发端已有 gateway）—非控制台

## 开发约束（所有新 sideigations 都适用的规则）

1. `.wt/<feature>` 使用 worktree（不删不动其他会话目录）。
2. 动目录里删除东西需要用 `gio trash` 而不是 `git clean`。
3. 妄图搞 `git reset` / `clean` / `--force` 之前先问。
4. 任何 PR 要 `cargo check` 零错，且主线测试（-p auth, -p catalog, -p admin-api-router）过。

## 关键风险记录

- **P0不先做就会出车祸**：现阶段全站观众（站主）新创建渠道/token 在控制台还能看到，但 gateway 转发换不来服务（`/v1/*` 认证不走 api_tokens 认证）。这就是迫在眉睫的整合点，也是 P0。
- **when later might be never**：本路线图已明确抄写进度。比如 OAuth / 2FA ≠ 不做。加注到技术债务账里，等核心闭环跑完再排。
