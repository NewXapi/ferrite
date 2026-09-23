---
name: notes
description: '查找并回写源码注释里的三个标记：TODO（维护者派活）/ ASK（维护者的问题·想法）/ DONE（agent 已完成）。用 rg 快筛 + ast-grep 精筛 + 符号绑定，一条命令拿到「维护者在哪里写了什么」，含只看「这轮新加的」。适用场景：用户说「看我写的批注 / 我标了 TODO 的地方 / 我的问题在哪 / 这轮我标了什么」，或 agent 要在开工前读维护者的要求、干完后回写 DONE。'
license: MIT
---

# notes —— 三个标记，维护者与 agent 的沟通通道

维护者在注释里写 `TODO` / `ASK`，agent 读它们干活，干完回写 `DONE`。
工具**只读**（写标记就是直接在注释里写），一条命令拿齐，不用翻文件、不用猜关键词。

## 语法

```rust
// TODO(hathaway): 扣额度必须走同一事务入口
pub fn charge(user: &str, n: u64) -> u64 { … }

// ASK(hathaway): 配额要不要按分组限？我倾向按分组
// TODO(#801): 探活定时调度还没接
// DONE(#801, hathaway): 探活调度已接 in=4f2a1c9 by=agent
```

| 标记 | 谁写 | 含义 | 对面该做什么 |
|---|---|---|---|
| `TODO` | 维护者 | 派给 agent 的活 | agent 做，做完改成 `DONE` 并补 `in=<commit> by=<agent>` |
| `ASK` | 维护者 | 问题 / 想法 | agent 在标记**下一行**注释里写「答：…」；要维护者拍板的别自己决定 |
| `DONE` | agent | 已完成留痕 | 维护者验收后自行删除 |

- 标记**必须大写**且紧跟 `(`，正文里出现小写 `todo` 不会误报。
- 括号里是 meta（逗号分隔）：`#801` = issue，`hathaway` = 谁写的。
- `)` 之后是正文；同一行再后的 `key=value` 会解析成字段（`in=`/`by=`/`see=`/`sym=`…）。
- **长正文 / 伪代码**：紧跟标记的连续注释行都属于正文，`--body` 整段打印。
- 旧写法不受影响：仓库里大量 `// TODO(#213): …` 原样能扫。

## 命令

```bash
N=.agent/skills/notes/notes

$N                      # 三个标记全列
$N --tags TODO,ASK      # 只看"还没做完的"（维护者派的活 + 待答问题）
$N --tag DONE           # 只看 agent 已完成的
$N --body               # 连正文一起打印（回答、伪代码整段）
$N --new                # 增量：只看这轮新加的（游标 .git/notes-cursor）
$N --since main         # 只看 main 之后新增行里的标记
$N --owner hathaway     # 只看某个人写的
$N --issue 801          # 只看挂了某个 issue 的
$N --path crates/gateway
$N --json               # 结构化（含 symbol{name,signature,line}）
$N --marker 'ponytail:' # 临时扫别的字面量标记（不限这三个）
$N --fast               # 只跑 rg，跳过 ast-grep 精筛
$N --all-tags           # 打印标记表
```

## 两条标准流程

- **agent 开工**：`$N --tags TODO,ASK --body` → 先把维护者派的活和问题读全；改代码前留意标记绑定的符号（输出里的 `→ charge`）。
- **agent 收工**：把做完的 `TODO` 就地改成 `DONE`，补 `in=<commit> by=<agent>`；
  回答 `ASK` 就在它下一行写 `答：…`。然后维护者用 `$N --new` 只读增量。

## 为什么不是裸 grep（三层，脚本已内置）

1. **快筛** `rg --json`：一次扫全仓，拿 file/line/字节偏移（全仓 0.2s 量级）。
2. **精筛** `ast-grep`：只留**注释节点**里的命中 —— 裸 grep 会把字符串字面量里的
   `"// TODO(x): …"` 一起捞出来（实测：精筛后 0 命中）。
3. **取用** `ast-grep outline` + 字节偏移：正文按偏移读（重构后不漂），并把标记绑定到所属符号。

## 陷阱（实测踩过，别"优化"掉）

1. 注释节点 kind 按语言不同：Rust 是 `line_comment`/`block_comment`，Python/TS/Go/C 等是 `comment`。
2. 精筛失败**不做减法**（未知扩展名/解析器缺失时保留 rg 结果），否则整批标记会消失。
3. `ast-grep` 的 `follows`/`precedes` 关系规则不可靠（紧邻判定会被中间那行 `///` 打断），绑定在脚本里做。
4. 本机 CLI 名是 `ast-grep`，没有 `sg` 短名。
5. 游标在 `.git/notes-cursor`：不进工作区、不打扰 `git status`。

## 复用到别的仓库

整目录拷 `.agent/skills/notes/` 过去即可（脚本零仓库耦合，只依赖 `rg`、`ast-grep`、`python3`）。

## 与 gate 的关系

本 skill 只读；`TODO(#N)` 仍受仓库既有 gate（`rust_todo_needs_issue`）约束。
要更严可另配 `.githooks/spec/checklist_*.yaml`：`owner` 必填、`DONE` 必须带 `in=`/`by=`、
`sym=` 悬空检测 —— 见 `skill://gate-checklist`。
