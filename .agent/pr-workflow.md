# PR 开发流程（主控 / 子代理编排）

> 从 `AGENTS.md` 抽离的八阶段编排剧本。硬性门禁摘要保留在根 `AGENTS.md`「PR 开发流程」节；开 PR / 派子代理编排开发前必须读本文。

你是主控 agent：编排任务、派子代理执行、审查子代理产出，**不要亲自把核心实现写完**。

## 硬性门禁

- **PR-only**：一切工作面以 PR 登记（见根 AGENTS.md「开发方式」）；禁止新建 GitHub issue、禁止改 epic 结构（挂/摘 sub-issue）。仅当用户 prompt 明确要求建 issue 时，先报备标题与 done when，批准后才建。
- **工作目录门禁**：所有子代理必须在 `.wt/<branch>` 工作；子代理 prompt 必须写明**全局绝对路径**（如 `/home/hathaway/projects/ferrite/.wt/<name>/`）与所属分支，限定其只在该目录内读写、编译、提交；禁止在仓库根目录或其他 worktree 落文件。
- **任务量门禁**：单个子任务 ≤ 5 个文件、单一主题、单一修改范围；能按文件 / 范围 / 主题 / 调用链 / 测试拆就拆，不把半个模块丢给一个子代理。
- **登记处**：suspect area 与风险点写进 PR body 对应字段（不进 done when）；子任务以 checkbox 形式登记到 PR body 任务清单，完成勾回。
- 每轮「审查 + 修复」写 **一条** PR comment（含修复 commit SHA）；smoke 验证再单独写 **一条** comment，说明验证手段与结果。两种留言可能多次出现。
- 不绕过 `.githooks/` 拦截门，不绕过 `hooks/merge --dry-run` 的预检。

## workflow（按阶段执行）

### 0. setup

- 从目标 base 拉 `<branch>`，工作树放 `.wt/<branch>`；不在仓库根目录改。
- 开 draft PR：conventional title，body 含目标 / 范围 / 任务清单 checklist / 验收命令。
- 开工前查最近 24h 内相关在跑 PR / 会话；工作面重叠时停下问用户。
- 用 `todo` / `goal` slash command 登记开发目标与阶段，主控和子代理全程对照确认，偏航即纠正；PR body 任务清单与之同步。
- 记录 `base_sha`，后续 CRG / diff review 用 `--base <base_sha>`，不要写死 `main`。

### 1. scope

- 跑一次 `code-review-graph update` / 取图谱。
- 修改导出符号前必须查引用（用 codegraph 图谱查调用方，本地没有 LSP）。
- 找到本次要动的模块、调用方、被调用方、相邻边界。
- 输出：suspect area（写进 PR body）、风险点、可能波及的文件清单。

### 2. break down

- 先按文件拆；同文件内再按修改范围拆；仍然太大就按主题 / 调用链 / 测试拆。
- 每个子任务必须写清：全局绝对路径 cwd、允许修改的文件、禁止触碰的文件、goal、非目标、验收命令（哪条命令跑通 = 完成）。
- **开发必须带测试**：新功能/修复的同一 PR 里补测试，尽可能覆盖完整场景（正常路径、边界、错误输入、并发/重入）；当前无法覆盖的场景用占位宏 `todo!("TODO(#N): 场景说明")` 显式声明纰漏。测试代码同样要写注释：说明测的是什么行为、为什么是这个预期。
- 不相信子代理会自动完成：每个子任务都要有主控可复验的 diff 边界和验收证据。
- 子任务太大、文件边界不清、或需要跨模块协调 → 继续拆；禁止「一个子代理干完半个模块」。
- 子任务登记到 PR body 任务清单，方便后续 closeout 勾回。

### 3. dev → audit

```
loop1:
  dev   → 派子代理按划分任务做，最多并行 2 个互不冲突子任务；同文件 / 同模块写入必须串行
  audit → 子代理完成后，主控（你）独立校验：
          - 跑子代理提供的验收命令（真跑，不只看输出）
          - diff 看改动是否只落在声明的文件
          - 检查 root cause、调用方、边界输入
          - 必要时再派一个校验子代理做交叉 confirm
失败 → 重拆或回 dev
```

### 4. test

- 全部子任务通过 audit 后，本地仅运行极小范围的类型检查（`cargo check -p <crate>`，CPU-heavy 必须套 `cpulimit -l 65 -i --`）。
- **尽可能不要在本地运行 `cargo test`**：所有集成测试、多 crate 联调与重型测试一律推送到 PR 分支，交给 GitHub CI 依据 `git diff` 动态按需执行。
- **重型测试**（>2 min、需要容器 / 网络 / 大数据）放 CI；CI 未跑完前不得 closeout / merge。
- **CI 驱动闭环**：以 GitHub CI 运行报告为准；CI 未全部跑绿前不得 closeout / merge。
- 若 CI 报错失败 → 提取云端失败日志回 loop1，把失败当作新子任务进行精准修复。
- 本地调试单个失败用例：`cargo test -p <crate> -- <test_name>`（仅调试，不替代 CI 验收）。

### 5. tool review

- 先 CRG（结构层）：`code-review-graph detect-changes --base <base_sha>`。
- 再 ocr（规范层）：`ocr review --from <base_sha> --to <ref>`（OpenCodeReview 代码审查，非截图 OCR）；**按模块、按 PR diff 分块喂**，不要一次性 send all（限流）。
- 发现 bug / problem → 回 loop1 修复 → 重新 review，直到干净。
- 每轮（review + fix）→ 1 条 PR comment（含每轮发现、修复 commit、验证命令）。

### 6. smoke

- 真实用户路径跑一遍：CLI 命令 / 真实 URL / 真实进程；UI 截图或 OCR 对比。
- 发现问题 → 更新 PR 任务清单 → 回 loop1 做二次修复。
- 通过 → 在 PR 写一条「smoke 验证通过 / 用的方法 / 结果」comment。

### 7. tidy

- **gate 复检**：跑 `gate pre-commit` / `gate pre-push` 全量规范检查，作为 tidy 的第一道清单；FAIL 清零再进下面各项。
- **file/dir**：检查分支目录里有没有跟本次开发无关的杂物（旧脚本、临时文件、废弃产物），要么加 `.gitignore`、要么用 `gio trash` 移入回收站（严禁 `rm` 或 `git clean` 永久删除）。
- **code**：测试代码没放 `tests/` 的挪过去；`cargo fmt` / `prettier` / 项目对应 formatter 跑一遍；无调试 log、commented-out code、调试 surrogate；rust doc 与实现不一致的更新掉；formatter 如修改文件，必须重跑最小验收命令、tool review、smoke，并更新 PR comment。
- **docs**：同步改动的代码注释、`AGENTS.md` / `README.md` / `docs/` 里过期的段落，引用跟新增要一致。

### 8. report

- report: 改了哪些文件、跑了哪些测试、CRG / ocr / smoke 的结果、PR 链接、剩余风险。
- **收尾报备**：列出本会话新建/修改的全部 PR；有未报备的新建即违规。
