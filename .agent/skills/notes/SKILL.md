---
name: notes
description: '按标记读/写**源码注释**：notes 命令列出维护者写的 TODO（派活）与 ASK（问题·想法），agent 干完把标记改写成 DONE。用于代码，不是网页 UI 批注（网页批注用 ainotation-web 技能）。适用场景：用户说「看我写的批注 / 我标了 TODO 的地方 / 我的问题在哪 / 这轮我新加了什么」，或 agent 开工前要读维护者的要求、收工回写 DONE。'
license: MIT
---

# notes —— 源码注释里的三个标记（TODO / ASK / DONE）

维护者在注释里写 `TODO` / `ASK`，agent 读它们干活，干完**回写 `DONE`**。工具只读，写标记就是直接在注释里写。

## 语法与三个标记

```rust
// TODO(hathaway): 扣额度必须走同一事务入口
pub fn charge(user: &str, n: u64) -> u64 { … }

// ASK(hathaway): 配额要不要按分组限？我倾向按分组
// TODO(#801): 探活定时调度还没接
// DONE(#801, hathaway): 探活调度已接 in=4f2a1c9 by=agent
```

| 标记 | 谁写 | 含义 | 对面**必须**做什么 |
|---|---|---|---|
| `TODO` | 维护者 | 派给 agent 的活 | 做完**就地改成 `DONE`**：`DONE(<原 meta>): <一行结论> in=<commit 或 wip> by=<agent>` |
| `ASK` | 维护者 | 问题 / 想法 | 在标记**下一行**注释写 `答：<答案>`；要维护者拍板的**保持 `ASK`** 并写明要他确认哪一点 |
| `DONE` | agent | 已完成留痕 | 维护者验收后自己删 |

回写模板（照抄，别只在聊天里说）：

```rust
// TODO(#801, hathaway): 探活定时调度还没接        ← 维护者写的
// DONE(#801, hathaway): 探活已接（ops::probe 定时触发） in=4f2a1c9 by=agent   ← agent 改成的

// ASK(hathaway): 配额要不要按分组限？             ← 维护者写的
// 答：建议按用户维度限流（60/min）；需要你确认是否再按 IP 兜底        ← agent 补的下一行
```

- 标记**必须大写**且紧跟 `(`；正文里写小写 `todo` 不会误报。
- 括号里：`#801` = issue、`hathaway` = 谁写的；`)` 之后是正文；同一行再往后的 `key=value`
  解析成字段（`in=` `by=` `see=` `sym=`…）。
- **长正文 / 伪代码**：紧跟标记的连续注释行都属于正文，`--body` 整段打印。
- 旧写法不用迁移：仓库里 `// TODO(#213): …` 原样能扫。

## 命令

## 两条流程（就这两条）

- **开工**：`$N --task` → 待办清单 + 回写格式（工具会把格式再打一遍，照抄）。
- **收工**：按上面的模板把每条 `TODO` 改成 `DONE`、给每条 `ASK` 补 `答：`；
  然后 `$N --lint` 自检（`DONE` 缺 `in=`/`by=`、`ASK` 没写「答：」都会被抓出来，退出码 1）。
  **没做完的不要标 `DONE`**；没提交就写 `in=wip`，提交后再补短号。

## 命令

```bash
N=.agent/skills/notes/notes

$N --tags TODO,ASK --body   # 维护者派的活 + 待答问题
$N --new                    # 只看这轮新加的（游标 .git/notes-cursor）
$N --tag DONE               # 只看 agent 已完成的
$N --json                   # 结构化（含 symbol{name,signature,line}）
$N --all-tags               # 标记表
$N --task                   # 待办清单 + 回写格式（开工先跑这条）
$N --lint                   # 协议自检：DONE 缺 in=/by=、ASK 没答 → 退出码 1
```

其余参数：`--owner` `--issue` `--sym` `--path` `--since <ref>` `--body` `--fast` `--marker '字面量'`。

内部三层，别拆：`rg` 快筛拿字节偏移 → `ast-grep` 精筛只留**注释节点**（裸 grep 会把字符串里的
`"// TODO(x): …"` 一起捞出来）→ `ast-grep outline` 把标记绑到所属符号（输出里的 `→ charge`）。
精筛失败时脚本保留 rg 结果、不做减法 —— 否则那种语言的标记会整批消失。

## 陷阱

1. 注释节点 kind 按语言不同：Rust 是 `line_comment`/`block_comment`，Python/TS/Go/C 等是 `comment`。
2. `ast-grep` 的 `follows`/`precedes` 关系规则不可靠（紧邻判定会被中间那行 `///` 打断），
   绑定放在脚本里做。
3. 本机 CLI 名是 `ast-grep`，没有 `sg` 短名。
4. 游标写在 `.git/notes-cursor`：不进工作区、不打扰 `git status`。

拷到别的仓库：把 `.agent/skills/notes/` 整个目录拷过去即可（零仓库耦合，依赖 `rg` + `ast-grep` + `python3`）。
`TODO(#N)` 这类注释仍受仓库既有 gate（`rust_todo_needs_issue`）约束。
