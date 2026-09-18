# PR 开发流程（主控 / 子代理编排）

> **本文按「问题 → 意图 → 情况 → 约束」四段组织**；开 PR / 派子代理编排开发前必读。
> **按阶段阅读索引**：先读完本文；stage 4 → `.agent/playbook/testing-ci.md`；
> stage 7 与任何 `gh` 操作 → `.agent/playbook/gates.md`（被拦查规则：`.githooks/GATE_HANDBOOK.md`）；
> 起 dev 环境 → `.agent/playbook/dev-env.md`；写 UI / Rust / 调查代码 → `.agent/playbook/conventions.md`。

## 1. 遇到什么问题

| 问题 | 后果 |
|---|---|
| 主控亲自把核心实现写完 | 子代理变成摆设，审查失去独立性 |
| 子代理在错误的目录里干活 | 污染仓库根 / 落进别人的 worktree |
| 任务切得太大（「半个模块」） | 子代理产出不可验收，返工 |
| 只在最后跑一次审查 | 问题堆积，修复成本指数上升 |

## 2. 维护者希望做什么事

你是**主控** agent：编排任务、派子代理执行、审查子代理产出，**不要亲自把核心实现写完**。

- 一切工作面**以 PR 登记**；子任务以 checkbox 形式登记到 PR body 任务清单，完成勾回。
- 每轮「审查 + 修复」写 **一条** PR comment（含修复 commit SHA）；smoke 验证单独再写 **一条**。
- suspect area 与风险点写进 PR body 对应字段（不进 done when）。

## 3. 可能的情况（八阶段）

### 0. setup

- 从目标 base 拉 `<branch>`，工作树放 `.wt/<branch>`；**不在仓库根目录改**。
- 开 draft PR：conventional title，body 含目标 / 范围 / 任务清单 checklist / 验收命令。
- 开工前查最近 24h 内相关在跑 PR / 会话；工作面重叠时停下问用户。
- 用 `todo` / goal 登记开发目标与阶段，主控和子代理全程对照，偏航即纠正；与 PR body 同步。
- 记录 `base_sha`，后续 CRG / diff review 用 `--base <base_sha>`，不要写死 `main`。

### 1. scope

- 跑一次 `code-review-graph update` / 取图谱。
- 修改导出符号前**必须查引用**（用 codegraph 图谱查调用方，本地没有 LSP）。
- 找出要动的模块、调用方、被调用方、相邻边界。
- 输出：suspect area（写进 PR body）、风险点、可能波及的文件清单。

### 2. break down

- 先按文件拆；同文件内再按修改范围拆；仍太大就按主题 / 调用链 / 测试拆。
- 每个子任务写清：全局绝对路径 cwd、允许修改的文件、禁止触碰的文件、goal、非目标、验收命令。
- **开发必须带测试**：同 PR 内补测试，覆盖正常路径 / 边界 / 错误输入 / 并发重入；
  当前无法覆盖的场景用 `todo!("TODO(#N): 场景说明")` 显式声明纰漏。测试代码同样要写注释说明预期。
- 不相信子代理会自动完成：每个子任务都要有主控可复验的 diff 边界和验收证据。
- 子任务太大、边界不清、需跨模块协调 → 继续拆；禁止「一个子代理干完半个模块」。

### 3. dev → audit（loop）

```text
loop1:
  dev   → 派子代理按划分任务做，最多并行 2 个互不冲突子任务；同文件 / 同模块写入必须串行
  audit → 子代理完成后，主控独立校验：
          - 跑子代理提供的验收命令（真跑，不只看输出）
          - diff 看改动是否只落在声明的文件
          - 检查 root cause、调用方、边界输入
          - 必要时再派一个校验子代理做交叉 confirm
失败 → 重拆或回 dev
```

### 4. test

- 全部子任务通过 audit 后，本地仅跑极小范围类型检查（`cargo check -p <crate>`，套 cpulimit）。
- 集成测试、多 crate 联调、重型测试（>2min / 需容器 / 需网络）一律推 PR 交 CI。
- CI 报错 → 提取云端失败日志回 loop1，当作新子任务精准修复。
- **注意**：新增测试要确认 CI 真会执行它（feature-gate / PG skip / `#[ignore]` 三个静默失效模式见 `testing-ci.md` §3.3-3.5）。

### 5. tool review

- 先 CRG（结构层）：`code-review-graph detect-changes --base <base_sha>`。
- 再 ocr（规范层）：`ocr review --from <base_sha> --to <ref>`；**按模块、按 PR diff 分块喂**，不一次性 send all（限流）。
- 发现 bug / problem → 回 loop1 修复 → 重新 review，直到干净。
- 每轮（review + fix）→ 1 条 PR comment（含发现、修复 commit、验证命令）。

### 6. smoke

- 真实用户路径跑一遍：CLI 命令 / 真实 URL / 真实进程；UI 用截图或 OCR 对比。
- 发现问题 → 更新 PR 任务清单 → 回 loop1 二次修复。
- 通过 → 在 PR 写一条「smoke 验证通过 / 用的方法 / 结果」comment。

### 7. tidy

- **gate 复检**：`gate pre-commit` / `gate pre-push` 全量规范检查，FAIL 清零再进后续项。
- **file/dir**：清掉与本次开发无关的杂物（旧脚本、临时文件、废弃产物）——加 `.gitignore` 或 `gio trash`
  （**严禁 `rm` / `git clean`**）。
- **code**：测试代码没放 `tests/` 的挪过去；跑 formatter；无调试 log / 注释掉的代码 / 调试 surrogate；
  rust doc 与实现不一致的更新掉。formatter 改了文件 → 重跑最小验收 + tool review + smoke，并更新 PR comment。
- **docs**：同步代码注释、`AGENTS.md` / `README.md` / `docs/` 里过期段落，引用与新增一致。

### 8. report

- 报：改了哪些文件、跑了哪些测试、CRG / ocr / smoke 结果、PR 链接、剩余风险。
- **收尾报备**：列出本会话新建/修改的全部 PR；未报备的新建即违规。

## 4. 约束事项（硬性门禁）

- **PR-only**：一切工作面以 PR 登记；禁止新建 GitHub issue、禁止改 epic 结构（挂/摘 sub-issue）。
  仅当用户 prompt 明确要求建 issue 时，先报备标题与 done when，批准后才建。
- **工作目录门禁**：所有子代理必须在 `.wt/<branch>` 工作；子代理 prompt 必须写明**全局绝对路径**
  （如 `/home/hathaway/projects/ferrite/.wt/<name>/`）与所属分支，限定其只在该目录内读写/编译/提交；
  禁止在仓库根目录或其他 worktree 落文件。
- **任务量门禁**：单个子任务 ≤ 5 个文件、单一主题、单一修改范围。
- 不绕过 `.githooks/` 拦截门，不绕过 `hooks/merge --dry-run` 预检。
