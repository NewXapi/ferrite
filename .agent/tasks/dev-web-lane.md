# 任务书：web 开发车道（dev 角色）

**什么时候用**：web 域（`crates/web/*`、`apps/admin-web`、`apps/tavern-web`）的快速改动。
流程总规则读 `.agent/rules/web-lanes.md`；本任务书只讲开发角色怎么跑。

**角色定位**：敏捷开发。不派子代理、不做 CRG、不写 PR 评论、不开 PR——
改完 squash 合回 `web-dev` 就结束。审查是另一个角色（`webfix-lane.md`）的事。

---

## 0. 开工（每个会话一次）

```bash
export CARGO_TARGET_DIR=/home/hathaway/projects/ferrite/target   # 共享主检出编译目录；仅限与 web-dev 内容一致的会话
                                                                 # 并行第二实例/带未提交改动必须换独立目录
                                                                 # （如 target-web-parallel），否则产物互覆盖
                                                                 # + mtime 竞争 → 对方 dx serve 到你的旧 wasm
cd /home/hathaway/projects/ferrite/.wt/web-dev                        # 全局绝对路径，别自己推路径
git status                       # 必须干净
git checkout web-dev && git pull --ff-only
git merge refs/heads/main                   # 同步干线：把其他域合进 main 的提交吃进来（单向，只此一次）
git checkout -b feat/xxx         # xxx = 短名，如 wheel-tab-fix
```

**先读**（第一次进 web 车道必读，后续会话可跳）：`.agent/rules/gates.md`、
`.agent/rules/web-lanes.md`、`.agent/skills/ainotation-web/SKILL.md`。

## 1. 环境（后端 + 前端 + 标注栈，按序起）

```bash
# 后端：共享 3211，常驻。没起才从主检出拉（不要在 .wt 里起）：
cd /home/hathaway/projects/ferrite && just dev-backend status || just dev-backend start
curl -s -o /dev/null -w '%{http_code}\n' http://127.0.0.1:3211/api/dashboard   # 401 = 活着

# 标注栈（硬顺序：service 先于 bridge 先于前端；全用运行时托管后台任务，禁止 nohup &）：
# 本例端口 8092（8090 被占；换端口时下面全文替换，bridge 的 AINO_ORIGIN 必须同步改）
just aino-service                 # 就绪判据 ~/.ainotation/service/connection.json
AINO_ORIGIN=http://127.0.0.1:8092 AINO_DIRECTORY=/home/hathaway/projects/ferrite just aino-bridge   # 同步桥 :44090
just dev-web 8092 debug           # 免登录自动登 admin_dev；起后 ss -ltn 验证 8092 在听
just aino-check 8092              # 体检：service + 桥 + :8092 前端三绿
```

端口被占就换（8091/8092…），**换端口必须同步改 bridge 的 `AINO_ORIGIN`**，并在汇报里写实际端口。
只用普通预览不起标注栈时：`just dev-web <port>`。

**标注栈三个硬约束（实测踩过）**：

- **一次只服务一个 origin**：页面 SDK 端点端口写死 `127.0.0.1:44090`，grant 绑 origin。
  别的会话（含主检出 8090）占着桥时，本车道页面的标注**静默不工作**——要么协商停掉
  对方的桥用本车道端口重启，要么本车道只开发、不发标注。
- **车道里起桥必须带 `AINO_DIRECTORY=/home/hathaway/projects/ferrite`**：否则项目按
  worktree 路径注册，agent 的 MCP（connect 的是主检出目录）**读不到**车道里的标注。
- service 是共享单例（`just aino-service` 幂等）；MCP 读标注不经过桥——只读标注的
  会话（如 webfix 车道）不起桥。

## 2. 开发循环（改 → 一条命令 → 强刷）

```bash
# 改代码（含 ui-components 等依赖 crate，dx 不会自动重编）
just dev-web-rebuild 8092 debug   # 一键：杀旧 dx → 原档位重启（dx 启动时自会重编 wasm）
# 浏览器强刷一次（wasm/js 有缓存）；功能不对就继续改，循环同上
```

实测耗时（共享 target 的 `.wt` 车道）：新 worktree 首建 ~54s，无改动重启 ~20s，
带改动重启取决于改动面。改一次 = 一条命令 + 一次强刷，分钟级。

用户的视觉意见来自 ainotation：MCP 工具 `ainotation_get_feedback` /
`ainotation_get_image` 读标注（comment 是原话、`targets[].selector` 直接定位代码）。
`just aino-bundle` 只在改了 `ainotation-entry.ts` / 升级 SDK 后才需要，打完必须重启 dx。

## 3. 收尾（合回 web-dev）

```bash
git add <本次改动的文件>           # 只 add 自己改的，不顺手 stage 别人的
git commit -m "feat(web): <一句话>"   # conventional；body 写「为什么改、用户原话/现象」
                                       # 审查者没有你的对话上下文，commit message 是他的唯一意图来源
git checkout web-dev && git merge --squash feat/xxx
git commit -m "feat(web): <同上的干净 message>"
git push origin web-dev && git branch -d feat/xxx
```

**commit 闸门两个坑（实测踩过）**：

- **陈旧 `COMMIT_EDITMSG`**：pre-commit hook 在 git 写入新 `-m` 消息**之前**就读
  `.git/COMMIT_EDITMSG`——上一个手工/merge commit 的旧消息会被当成你的验，报 phantom
  CM-01 FAIL。提交前先刷新：`printf '<你的消息>\n' > .git/COMMIT_EDITMSG` 再
  `git commit -F .git/COMMIT_EDITMSG`（或先 `git commit --dry-run -m "..."` 刷文件）。
- **main 上标题必须英文**（CM-02 在 main 是 FAIL，在车道分支只是 WARN）：中文标题在
  feat/webfix/web-dev 上随便写，发布 PR 的 commit（含 squash 进 web-dev 的）用英文。

推之前本地过闸门（pre-commit/push 钩子会自己跑；主动预检）：

```bash
gate check                        # FAIL 必须清零再 push，WARN 说明理由后可放行
```

**新 crate 必须同时建 README**（doc_sync FAIL）；组件被第二个 page 使用时搬进
`crates/web/ui-components`（shared_components_check FAIL）。

## 4. 汇报格式（每轮结束）

- 当前分支 / feat 分支名 / 实际端口
- 产生了哪些 commit（SHA + message）
- 已 squash 合回 web-dev 并 push（web-dev 新 tip SHA）
- ainotation 标注处理情况（读了哪条、改了没）
- 冲突或未完成事项

## 禁止

- 禁止派子代理 / 建 PR / 写 PR 评论 / 跑 CRG（那是 webfix 角色和发布流程的事）
- **web 改动的落点只有一个：`.wt/web-dev`**（车道固定 worktree，初始化时已建好）。
  禁止为单个改动另开 `.wt/<xxx>` 一次性 worktree（每开一个付一份冷 wasm target），
  也禁止在仓库根目录或其他会话的 worktree 落文件
- 禁止 `pkill -f cargo` / `pkill -f rustc` / `rm` / `git clean`；删文件用 `gio trash`
- 禁止停共享 3211 后端、禁止对共享库 `db-reset`
- 禁止 force push 任何共享分支
