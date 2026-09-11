# `db/dev` — 共享开发数据与后端

让所有 `.wt` 开发会话用**同一份**有真实质感的数据和**同一个**后端实例，
不再各自手动造数、不再互相抢 3211 端口。

## 快速开始

```sh
just db-seed            # 灌种子 (幂等, 3s)
just dev-backend start  # 起共享后端 (127.0.0.1:3211)
```

dev 管理员账号: `admin_dev / DevPassw0rd!12345` (role=100)。
各 worktree 的 admin-web (`dx serve`) 代理默认已指向 3211，直接起前端即可。

## seed.sql 不入库

由 `generate_seed.py` **现场生成、管道直通 psql**（不落盘，`.gitignore` 已忽略）。
生成器的数据源是 **`texture.json.gz`（57KB，已入库）** —— new-api 运行库
consume 日志的压缩纹理，**不要手改纹理与生成器**：

- **真实纹理**：`texture.json.gz` 收录 3312 条真实 consume 日志（真实模型名
  glm-5.3-flash/claude-opus-5/…、真实 token 量级、quota、耗时、流式比例、昼夜节奏），
  由外部 new-api 运行库一次性提炼入库（重新提炼方法见本文件末尾）。
- **随机分布**：seed=42 的确定性 RNG 做时间重映射与用户分配，可复现——
  57KB 纹理生成的 SQL 与原 14MB 产物逐字节一致（diff=0）。
- **时间相对化**：时间戳全部是 `now() - interval` 表达式，提交到仓库后
  任何时间执行，今天/本周/本月/今年四个窗口都有数据，永不过期。
- **覆盖四张表**：`auth_users`（1 管理员 + 10 用户）、`api_groups`（default/vip）、
  `api_channels`（wt-52mxw 等真实渠道名 ×4）、`api_tokens`（×3）、
  `monitor_history`（4 渠道 × 320 条探活，可用率梯度 99.7/97.2/93/98.5%）、
  `usage_logs`（44,000 条 = 全年 30k + 近 7 天 8k + 近 24h 6k）。

### 幂等与清理

- `usage_logs` 种子行 `request_id LIKE 'seed-%'`；
- 账号/渠道/分组/令牌用固定 UUID（`00000000-…aa01/bb/cc/dd`），清理时
  **UUID 或唯一名**双条件删除，兼容历史手工创建的同名行；
- `just db-reset` 只删种子行，绝不碰真实数据；重复 `just db-seed` 先删后插。

## 共享后端实例（端口 3211）

**规则**：共享后端只从**主检出**（跟踪 `newxapi/main` 的 `/home/hathaway/projects/ferrite`）
运行；worktree 里开发的业务前端一律代理到它。后端代码本身的开发例外——
在 worktree 里用 `config/config.toml` 指定私有端口起自己的实例，不要动共享的。

```sh
just dev-backend status  # 健康检查
just dev-backend start   # 启动 (二进制缺失自动构建)
just dev-backend update  # 重建 + 重启
just dev-backend stop
```

### 更新协议（回答"运行中怎么更新"）

重启对前端**是透明的**，原因：

1. JWT secret 由 `FERRITE_JWT_SECRET` 环境变量固定（脚本默认
   `ferrite-local-development-secret-2026`），重启不换钥 → 已签发的
   access token 依然有效；
2. 会话在 PG（`auth_user_sessions`），不在进程内存 → 重启不丢登录态；
3. access token 有效期 dev 默认拉长到 12h（`FERRITE_JWT_ACCESS_TTL_SECS=43200`，
   代码默认 15min），配合 401 静默刷新，白天基本不会过期。

所以 `just dev-backend-update` = 重建二进制 + ~3 秒重启。其他会话唯一
感知到的是正在飞行中的那一个请求失败一次，页面上的"重试"按钮（或下一次
点击）即恢复。约定：更新前在会话里说一声即可，无需协调停机窗口。

环境覆盖：`FERRITE_DEV_LISTEN`（默认 3211）、`FERRITE_JWT_SECRET`、
`FERRITE_JWT_ACCESS_TTL_SECS`。

## 环境要求

- PG 容器 `uf-local-postgres`（库 `ferrite_smoke`），可在 justfile 顶部改
  `PG_CONTAINER`/`PG_DB`；
- 执行种子只需要 `python3` + `psql`（经 docker exec，宿主机无需装 psql）；
  纹理 `texture.json.gz` 已入库，无需外部 new-api.db。重新提炼纹理（罕见）：
  `python3 -c` 读外部库按 `generate_seed.py` 历史中的 `load_real_rows` 查询
  压缩回写 `texture.json.gz`，或参考 git 历史中的提取脚本。
