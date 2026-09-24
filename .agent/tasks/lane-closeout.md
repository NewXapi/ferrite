# 任务书：车道收尾（进程 / 分支 / 工作树 / 垃圾清理）

**什么时候用**：一次发布合并（`web-dev → main` 的 PR merge）之后，或维护者说「收尾」时，
由主控或专职收尾代理跑**一轮**。feat/webfix 短分支在各自循环里已随手清掉，不需要每轮跑本任务书。

**角色定位**：只清理，不开发。所有删除可逆（回收站 / git 层面），所有 kill 先验归属。
流程总规则读 `.agent/rules/web-lanes.md`。

---

## 规范（硬规则）

- kill 任何进程前必须 `readlink /proc/<pid>/cwd` 确认归属。只杀：本车道的、确认无主的
  （PPid=1 且 cwd 在本仓库）。活会话的进程不动；被 cpulimit 遗留 SIGSTOP 的 T 态进程用
  `kill -CONT` 恢复，**不 kill**。
- 禁止 `pkill -f cargo` / `pkill -f rustc` / `rm` / `rm -rf` / `git clean`；
  文件进回收站一律 `gio trash <path>`。
- **`.wt/web-dev` 和 `.wt/web-fix` 是车道固定家，收尾不删**——只有维护者明确说「车道退役」
  才动（见文末退役节）。
- 只停本车道的 dx / 标注桥；共享 3211 后端、其他会话的 dx / 桥（如主检出 8090）一律不碰。
- 拿不准归属的垃圾 / 改动：列进汇报，不动手。

## 0. 盘点（先出清单再动手）

```bash
cd /home/hathaway/projects/ferrite
git status -s                                        # 主检出未提交 / 未跟踪
git worktree list                                    # 全部 worktree
git branch -vv                                       # 本地分支（找已合并残留）
ss -ltnp | grep -E ':(809[0-9]|3211|44090|42241)'    # dx / 后端 / 桥 / service 端口
ps -eo pid,ppid,stat,etime,args | grep -E 'cargo|rustc|dx serve|ainotation-bridge|cli.mjs' | grep -v grep
for d in .wt/*/; do echo -n "$d dirty: "; git -C "$d" status -s | wc -l; done
du -sh target .wt/*/target 2>/dev/null | sort -rh | head -5
```

## 1. 进程清理

- **本车道 dx**：`ss -ltnp` 按端口取 pid → `readlink /proc/<pid>/cwd` 确认落在 `.wt/web-dev`
  或 `.wt/web-fix` → kill。origin 是其他端口的 dx 是别人的会话，不碰。
- **本车道标注桥**：pid 的 cwd / environ 确认 `AINO_DIRECTORY` 指向主检出、`AINO_ORIGIN`
  是本车道端口才 kill；origin 为 8090 的桥属于主检出会话，不碰。
- **孤儿 cargo / rustc**：PPid=1 + T 态 + etime 数小时 → `kill -9`；PPid 是 omp / 其他会话的
  → 不动（T 态 `kill -CONT`）。
- 运行时（hub 类）托管的长跑进程：优先用运行时的 stop 操作，不裸 kill。

## 2. 分支清理

```bash
git fetch --prune
git branch --merged refs/heads/web-dev | grep -E 'feat/|webfix/'   # 本地候选（已合入才列出）
git branch --merged refs/heads/main    | grep -E 'feat/|webfix/'   # 直提 main 的残留
# 逐个 git branch -d（能 -d 说明已合过；要 -D 的停下确认）
# 远端已合并的: git push origin --delete <branch>
git show-ref refs/heads/web-dev refs/heads/main    # 自检：两个干流分支必须健在
```

## 3. 工作树清理

- 分支已合并的一次性 `.wt/<xxx>`（不是 web-dev / web-fix）→ `git worktree remove <目录>`，
  **严禁 `rm`**；remove 报脏先 `git -C <目录> status` 看清再决定。
- `git worktree prune` 清失效元数据。
- 自检：`git worktree list` 里 `.wt/web-dev`、`.wt/web-fix` 必须还在。

## 4. 垃圾清理（主检出 + 各 worktree）

- `git status -s` 的未跟踪文件逐个判：
  - agent 临时产物（`tmp-*`、`*.orig` / `*.rej`、根目录散落脚本、`nohup.out`、测试输出）
    → `gio trash`；
  - 运行时生成且被引用的（如 ainotation `connection.json`）→ 保留；
  - 分不清归属 → 列进汇报不动。
- 未提交的 tracked 改动：本车道产的且已合流 → 核对无遗漏后恢复干净；**维护者自己的未提交
  改动原样保留，并在汇报里点名**（曾发生维护者 edits 和 agent edits 混在同一文件）。

## 5. 磁盘（可选，需维护者点头）

- 报告 `du -sh target .wt/*/target`；`just wt-candidates` 看可清项。
- `just wt-targets-clean` 会删编译产物（可重建但耗时）——只在维护者同意后执行。

## 6. 汇报格式

- 清了哪些进程（pid + 归属证据：cwd / PPid）
- 删了哪些分支（本地 / 远端）、移除了哪些 worktree
- trash 了哪些垃圾（路径）、保留了哪些及理由
- 仍需维护者决定的项（脏 target、拿不准的未跟踪文件等）

## 附：车道退役（仅维护者明确要求时）

```bash
git -C .wt/web-dev status -s && git -C .wt/web-fix status -s   # 先确认两边干净
git worktree remove .wt/web-dev && git worktree remove .wt/web-fix
git worktree prune
# web-dev 远端分支保留（发布历史在里面），不删
```
