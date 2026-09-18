# dev 环境启动与排错（后端 + 数据库 + 前端）

> 命令以 `justfile` 与 `scripts/dev-backend.sh` 为准；2026-09-18 实操验证过一遍。
> **本文按「问题 → 意图 → 情况 → 约束」四段组织**，AGENTS.md 只留触发条件。

## 1. 遇到什么问题（含曾经踩过的）

| 症状 | 先做 | 根因/出处 |
|---|---|---|
| 页面 500 "Connection refused" / dx 日志消失 | `just dev-check` 查 3211/8090 监听 | 服务被回收（常见于 `nohup &` 起的，工具调用结束即死） |
| 某卡片 404，但 curl / 无缓存浏览器实测 200 | dx 代理日志 grep 该路径：**查无请求 = 实锤** | 浏览器(IAB) 重放了代理误配期毒化的错误响应，请求根本没出网 |
| 构建卡死（rustc 长时间 0 进展 / cargo 锁等待） | `ps -eo pid,stat,args \| awk '$2 ~ /^T/'` 找 T 态 → `readlink /proc/<pid>/cwd` 认归属 → 无主才 kill，活会话用 `kill -CONT` | 别的会话构建被 cpulimit 遗留 SIGSTOP 僵进程持锁。**T 态 ≠ 死进程** |
| `just dev-web` 报 `Failed to find binary package to build` | 已修（2026-09-18）：配方曾用不存在的 `$(justfile_directory)` | just 1.58 无此变量，shell 展开空串 → `cd /apps/admin-web` 必失败 |
| dx 改了依赖 crate（ui-components 等）但页面没变 | `kill <dx pid>` 后重起 `just dev-web <port>` + 浏览器强刷 | dx 不自动重编依赖 crate 的 wasm |
| dx 首编很久（~336s）端口还没监听 | 等，别误判启动失败 | wasm 首次构建实测 336s |
| 改了 `crates/api` 前端行为没变 | `just dev-backend update`（~3s，重建+重启，**登录态不丢**） | 共享后端是长驻进程 |

## 2. 维护者希望做什么事

- 起/停/种子/体检**一律走 justfile 配方**，不手工拼命令：
  `just dev-check`（体检）· `just dev-backend start|update|stop|status`（共享后端）·
  `just db-seed` / `just db-reset`（种子）· `just verify`（fmt-check + clippy + check）。
  命令清单与场景见 justfile 顶部「使用场景速查」，疑难处置见其末尾「疑难问题 → 推荐处理」块。
- **开工前先 `just dev-check`** 一条命令自检环境。
- 长跑服务用**会话的持久后台任务**机制启动（不是 `nohup &`），起后 `ss -ltn` 验证端口在监听再交付。

## 3. 可能的情况

### 3.1 共享实例（默认，多会话共用）

| 步骤 | 命令 | 说明 |
|---|---|---|
| 前置 | `just dev-check` | 3211/8090 监听 + 后端状态 |
| PG 容器 | `docker ps --filter name=uf-local-postgres` | 必须 Up；没起 `docker start uf-local-postgres` |
| 灌种子 | `just db-seed` | 幂等可重跑；脏数据用 `just db-reset && just db-seed`（**db-reset 清共享库，跑前报备**） |
| 起后端 | `just dev-backend status` → 没跑再 `start` | 3211 是多会话共享实例，已运行就**不要 stop/restart** |
| 验证后端 | `curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:3211/api/dashboard` | **401 = 健康**（要鉴权）；refused = 没起 |
| 起前端 | `just dev-web 8090`（持久后台任务） | 起后 `ss -ltn` 验监听；`curl http://127.0.0.1:8090/` = **200** |
| 免登录调试 | `just dev-web 8090 debug` | 自动登录 dev 种子 `admin_dev`；打开 `#login`/`#signup`/`#auth` 仍可手动调登录页；主动退出不被自动重登顶掉 |

前端 500 refused 时 dx 的代理指向 3211：**先 `just dev-check` 看后端死活，别先动前端**。

### 3.2 隔离实例（独立校验，不碰共享）

需要独占种子数据 / 破坏性迁移时：

- `FERRITE_DEV_LISTEN=127.0.0.1:<port> scripts/dev-backend.sh start` 起独立端口
- 本 worktree 的 `config/config.toml`（gitignored、各 worktree 独立）DSN 指向另一个库
- 种子/重置用 just 变量覆盖：`just PG_DB=<你的库> db-seed`（justfile「PG 连接参数」注明容器/库可按环境覆盖）

**红线：严禁停共享 3211 后端、严禁对共享库 `uf-local-postgres/ferrite_smoke` 跑 db-reset。**

## 4. 约束事项（简略）

- 长跑服务禁用 `nohup ... &` 起（工具调用结束回收进程组 → 静默死）。
- 禁宽匹配 `pkill -f cargo/rustc`；清理前 `readlink /proc/<pid>/cwd` 认归属，只动无主残留。
- 共享 dev 后端生命周期只走 `scripts/dev-backend.sh`；发现 404/502 先判死活。
- 用户侧报错但 curl 实测全 200 → 先怀疑 IAB 缓存重放；服务端无法驱逐已毒化条目
  （只能用户清缓存/重启 webview）；后端 `/api`、`/tavern` 已加 `Cache-Control: no-store` 防复发。
- 历史坑（已修）：`just dev-web` 曾用不存在的 `$(justfile_directory)`，现为 `justfile()` 内置函数。
