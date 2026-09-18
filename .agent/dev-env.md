# dev 环境启动手册（后端 + 数据库 + 前端）

> 2026-09-18 实操验证过一遍。命令以 `justfile` 与 `scripts/dev-backend.sh` 为准；
> 本手册只给顺序、前置检查和排错入口，不重复 justfile 底部「疑难问题」块。

## 前置检查（一条命令）

```sh
just dev-check    # 查 3211/8090 监听 + 共享后端状态 + 进程卫生提醒
```

输出里若 3211 无监听且后端未运行 → 走下面的「后端 + 数据库」。

## 后端 + 数据库（共享实例，多会话共用）

| 步骤 | 命令 | 说明 |
|---|---|---|
| 1. PG 容器 | `docker ps --filter name=uf-local-postgres` | `uf-local-postgres` 必须 Up；没起就 `docker start uf-local-postgres` |
| 2. 灌种子 | `just db-seed` | 幂等可重复（2026-09-18 实测重跑无碍）；脏数据用 `just db-reset && just db-seed`（**db-reset 清共享库，跑前报备**） |
| 3. 起后端 | `just dev-backend status` → 没跑再 `just dev-backend start` | 3211 是**多会话共享**实例（所有 .wt 会话前端代理都指它），已运行就**不要 stop/restart**；改了 `crates/api` 代码用 `just dev-backend update`（重建 + 重启，登录态不丢，~3s） |
| 4. 验证 | `curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:3211/api/dashboard` | **401 = 健康**（端点存在、要鉴权）；Connection refused = 没起 |

### 独立校验用隔离实例（不碰共享）

- `FERRITE_DEV_LISTEN=127.0.0.1:<port> scripts/dev-backend.sh start` 起独立端口
- 本 worktree 的 `config/config.toml`（gitignored、各 worktree 独立）DSN 指向另一个库；种子/重置用 just 变量覆盖：`just PG_DB=<你的库> db-seed`
- **红线**：严禁停共享 3211、严禁对共享库 `uf-local-postgres/ferrite_smoke` 跑 db-reset

## 前端（admin-web）

| 步骤 | 命令 | 说明 |
|---|---|---|
| 1. 起服务 | `just dev-web 8090` | 必须用会话的**持久后台任务**机制跑（工具调用结束会回收进程组，裸 `&`/`nohup` 会静默死）；起完 `ss -ltn` 验证 8090 在监听 |
| 2. 免登录档 | `just dev-web 8090 debug` | 自动登录 dev 种子 `admin_dev`；直接打开 `#login` 仍可手动调登录页 |
| 3. 验证 | `curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:8090/` | **200** |
| 4. 后端连通 | 页面 500 "Connection refused" | dx 的代理指 3211；先 `just dev-check` 看后端死活，**别先动前端** |

### 前端排错速查（详见 justfile 底部「疑难问题」块）

- **改了依赖 crate（ui-components 等）页面没变**：dx 不自动重编 wasm。`kill <dx pid>` 后重新 `just dev-web <port>`，浏览器强刷。
- **某卡片 404 但 curl 实测 200**：浏览器/IAB 缓存重放了代理误配期的毒化响应（dx 代理日志 grep 该路径查无请求 = 实锤）；清 IAB 缓存或重启 webview。
- **dx 首编慢**：wasm 首次构建 ~336s（2026-09-18 实测），期间端口未监听是正常现象，别误判启动失败。

## 已知配方坑（本 PR 已修）

`just dev-web` 原配方 `cd "$(justfile_directory)"` 在 just 1.58 下必炸：`justfile_directory`
不是 just 内置（`just --evaluate` 报「justfile does not contain variable」），shell 展开为空串。
已改为 `cd "$(dirname "{{ justfile() }}" )/apps/admin-web"`（justfile 内置 `justfile()` 函数），
修复后完整跑通：dx 认到 admin-web 包 → wasm 构建 336s → 8090 监听 → 页面 200。
