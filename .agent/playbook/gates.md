# gate 操作细则（`.githooks/`）

> **本文按「问题 → 意图 → 情况 → 约束」四段组织**，AGENTS.md 只留触发条件。

## 1. 遇到什么问题

| 问题 | 现象 |
|---|---|
| 提交/推送被拦 | 输出 `checklist.*` / `WS-*` 格式的 FAIL / WARN 行 |
| 建 PR 被拒 | `闸门: 校验 FAIL，拒绝创建`（如「headings contain CJK」、「缺 type label」） |
| 以为改完了其实没看清 | 用 `\| head -5` 截断输出，漏掉后面的 FAIL |

## 2. 维护者希望做什么事

- 拦截信息**逐条读完再修根因**：禁止 `--no-verify`、禁止截断后忽略。
- **FAIL 必须清零**；WARN 说明理由后可放行。
- `gh` 操作在**创建时**就走 gate 校验：先跑预检，不等 push 才撞墙。
- 加规则只改 `.githooks/spec/checklist_*.yaml`，不改 gate 二进制。

## 3. 可能的情况

### 3.1 本地钩子（pre-commit / pre-push / merge）

- `gate pre-commit` / `gate pre-push`：tidy 阶段的第一道清单；FAIL 清零再进后续项。
- merge 前另有 `hooks/merge --dry-run` 预检，不绕过。

### 3.2 GitHub 侧校验（`gh` 操作）

- 输出 `FAIL` / `WARN` 两类行：
  - **FAIL 直接拦截**该 gh 操作 → 逐条修到清零再重试，不放弃、不绕过。
  - **WARN 不拦截但不要忽略**：每条说明缺什么（「缺 type label」「关键词建议挂某 label」「缺 `Fixes #`」）。
    能补就补（`gh pr edit --label` / `gh issue edit --label`），不能补的在 PR body 写明理由。
- 创建前先跑 `gate check` 对应清单或 `gate issue` / `gate pr` 预检。
- `head` / `tail` / `grep -v` 会吞掉提示行——**看 gate 输出务必完整读**。
- PR body 模板要求（实测）：标题/正文结构、中文 prose、**heading 必须英文**（CJK heading 会被 FAIL）、
  type label 至少一个（`bug`/`feature`/`chore`/`refactor`/`tests`/`documentation`/`epic`）。

### 3.3 规格文件在哪

- `.githooks/GATE_HANDBOOK.md`：一手总览——三层 SLA（l1 结构 / l2 语义 / l3 LLM）、
  16 条规则表（触发点 + 严重度）、「加规则只改 yaml」约定。
- `.githooks/spec/SPEC_OVERVIEW.md`：规则对照清单（**新增/修改规则后必须同步更新**）。
- 手动跑：`gate check [names...] --sla {l1|l2|l3} [--json]`。
- GitHub 侧规则：`.githooks/spec/github_pr_gates.yaml`、`checklist_pr_*.yaml`。

## 4. 约束事项（简略）

- 占位用 Rust 原生宏：`todo!("TODO(#<issue>): 说明")` 或 `unimplemented!(...)`；
  TODO/FIXME 注释必须带 issue 号（`TODO(#123): ...`）——`rust_todo_needs_issue` 检查项。
- 不绕过 `.githooks/` 拦截门；不用 `--no-verify`。
