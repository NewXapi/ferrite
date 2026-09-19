# web 页面 crate 重构规范

> 本文件是**规范**（单一权威），不是进度表。任何 agent 接手 `crates/web/*` 页面 crate 的重构都按本文件执行；
> 违反「硬约束」（§0）的改动一律不允许合入。
>
> 派单时用任务书 `.agent/tasks/restructure-web-page-crate.md`（填空版）。
> 推进状态（哪些 crate 已迁、剩余清单）见 `todo/web-page-refactor-rollout.md`。
>
> 适用范围：`crates/web/*` 页面 crate 与 `ui-components`。

---

## 0. 硬约束（不可协商）

以下四条是重构的前置条件，任何一条不满足就不许提交。

| 编号 | 约束 | 验收手段 |
|---|---|---|
| **H1** | **零行为漂移**：`data-testid`、`aria-*` 角色与标签、`class` 串、四态分支（加载 / 错误 / 空 / 有数据）、按钮文案与顺序，逐字保留 | `specs/ui/*.yaml` 的 ariaSnapshot 断言；§7 双重核验 |
| **H2** | **页面 rsx 只做组装**：页面文件（`page.rs`）里禁止写样式细节，禁止出现多行 `class="..."` 的视觉参数 | 人工审查；`page.rs` 每次改动都要重新判断 |
| **H3** | **组件内部不写死中文**：所有面向用户的文案走常量（§3） | grep 中文（§3.4） |
| **H4** | **公开组件必须有 6 要素注释**（§4） | 人工审查；缺一项即不合格 |

补充约束：

- 通用优先于重复：**同一段语义 class 串在 ≥3 处出现，必须抽成组件或常量**。
- 但**原子工具类不算重复**（`flex`、`gap-2`、`text-xs` 这类单个 utility 出现在 100 处也不用抽）——只抽**语义组合**（一整张卡的外壳、一个空态块、一个区段头）。
- 抽组件时**不改色值**：允许把 `border-zinc-800` 收敛成常量，**不允许**顺手换成设计 token（`border-border`）。色值迁移是独立任务，单独开 PR。

---

## 1. 目录结构

### 1.1 tab 页必须独占一个 `tab-page-<name>/` 目录

- 目录名格式：`tab-page-<name>`，`<name>` 用复数英文小写（`aliases` / `channels` / `groups` / `redemptions` / `subscriptions` / `keys` / `rewards` / `sessions` …）。
- 一个 tab 一个目录，**不允许**两个 tab 共用目录。
- 平铺的单文件页面（`src/aliases.rs` 这种）**不允许新增**；发现存量一律按本规范迁入目录。

**怎么发现一个 crate 有哪些 tab**：`ls crates/web/<crate>/src/`，平铺的 `<tab>.rs` 文件每个对应一个 tab；同时对照该 crate 的 `README.md` 文件清单。拿不准是不是 tab（区段 vs tab）时看 `lib.rs` 里的 `pub use` 与页面入口组件。

### 1.2 目录内文件职责固定

| 文件 | 职责 | 是否必需 |
|---|---|---|
| `mod.rs` | 声明子模块 + `pub use` 再导出；公开面只增不减 | ✅ |
| `page.rs` | 页面入口组件；持有跨组件状态；**薄 rsx 组装** | ✅ |
| `shared.rs` | 该 tab 独有的类型、常量、纯函数、文案常量 | ✅ |
| `stats.rs` | 顶部统计卡组（编号段 1） | 有则必拆 |
| `toolbar.rs` | 筛选 / 搜索 / 批量操作栏（编号段 2） | 有则必拆 |
| `list.rs` | 列表区（卡片网格 + 四态分支，编号段 3） | 有则必拆 |
| `modal.rs` | 编辑弹窗 | 有则必拆 |
| `<业务名>.rs` | 该 tab 独有的其他整块（如 `card.rs` / `form.rs` / `drawer.rs` / `inspector.rs`） | 按需 |

子组件文件**不带 tab 前缀**，按业务命名（`key_card.rs`、`topup_section.rs`）。

**页面里出现新块时的判定顺序**：

1. 是「统计 / 筛选栏 / 列表 / 弹窗」四类之一 → 用上表固定文件名。
2. 不是 → 看它是不是**整块独立 UI**（一段有明确边界、内部自成体系的 rsx）→ 是则新建 `<业务名>.rs`。
3. 还不够一整块 → 先留在 `page.rs`，等第二个 tab 也出现类似结构时再抽到 `ui-components`。

### 1.3 `lib.rs` 用 `#[path]` 桥接

目录名 kebab（`tab-page-keys`）与 Rust 模块名 snake（`tab_page_keys`）不同，靠 `#[path]` 连接：

```rust
pub mod api;

#[path = "tab-page-keys/mod.rs"]
pub mod tab_page_keys;
// …每个 tab 一组 #[path]

pub mod usage_support;

pub use tab_page_keys::KeysPanel;      // 公开符号名逐字保留（不改 KeysPage）
```

**公开面逐字不变**：`pub use` 的符号名保持和重构前一样。下游（`apps/admin-web` 只引用若干 Panel）与本 crate 测试（只引用 `api` / `usage_support`）都不该被动。

**怎么判定哪些符号属于「公开面」**：在仓库根 grep 谁 import 了本 crate 的符号——

```sh
grep -rn "use <crate_name>::" crates/ apps/ tests/
```

被 crate 外引用的 `pub` 符号就是公开面，名字一个字不能动；只在 crate 内部用的符号可以随拆分调整。

### 1.4 页面层文件统一叫 `page.rs`

不要叫 `panel.rs`。理由：跨 crate 重构时要按统一文件名定位页面层。公开组件名仍可叫 `XxxPanel`，二者互不影响——**文件名跟着本规范走（`page.rs`），符号名跟着 main 走**。

### 1.5 归属规则（决定一个组件放哪）

```
crates/web/ui-components/           ← 跨端复用，只依赖 contract
crates/web/<page-crate>/src/tab-page-*/   ← 只有这一个 tab 用
```

判断：**两个及以上 tab 需要同一组件 → 上移到 `ui-components`；只有一个 tab 用 → 留在 `tab-page-*/`。**

⚠️ `ui-components` **不许反向依赖任何 page crate**。若某类型原本定义在页面里但 ui-components 需要它（如 `PriceMode`），必须把类型**上移到 ui-components**，页面侧 `pub use` 回来（`tab-page-aliases/shared.rs` 的 `pub use ui::{PriceMode, PriceModeToggle};` 是范例）。

最终形态参考（已迁样板在 `.wt/web-visual` 的 `admin-page-admin`，10 个 `tab-page-*` 目录）：

```
crates/web/<page-crate>/src/
├── lib.rs                  # #[path] 桥接，公开面逐字不变
├── api.rs                  # *_api 真实调用（走 client::ApiClient）
├── usage_support.rs 等     # 呈现层纯函数共用库
└── tab-page-<name>/        # 一个 tab 一个目录，kebab-case
    ├── mod.rs              # pub mod 声明 + pub use 再导出
    ├── page.rs             # 页面层：跨组件状态 + use_hook 取数 + 薄 rsx 组装
    └── <块>.rs             # 各子组件，文件名无前缀
```

---

## 2. 组件抽象

### 2.1 什么该抽成 dioxus component

抽组件的判据是**「聚合度高的 rsx 子树」**，即同时满足：

- 子树有**明确的语义边界**（一张卡、一个空态、一个区段头、一行表单）；
- 内部有**自己的一组样式**；
- 被**复用了至少 3 次**，或虽然只出现 1-2 次但体积大（>30 行 rsx）且边界清晰。

**不为「将来可能复用」提前抽象**——出现第 3 个使用方时再抽。拿不准是不是「聚合度高」时，**不抽，留在 `page.rs`**。

### 2.2 三隔离

抽出来的组件必须做到：

| 隔离项 | 要求 |
|---|---|
| **样式隔离** | 视觉细节（class 串）封在组件内，用 `const` 提升复用；调用侧只传语义 props |
| **变量隔离** | 组件自用状态用组件内 `use_signal`；外部要读写才提升成 `Signal<T>` prop |
| **交互隔离** | 组件不直接调 API、不直接改外部状态；对外暴露 `EventHandler<T>` / `Signal<T>` |

### 2.3 状态归属规则（最常出错的点）

```
组件内部自用（不跨组件）        → 组件内 use_signal
外部需要读或写                 → 页面持有 Signal<T>，以 prop 传入
跨组件交互（筛选影响列表、
按钮开关弹窗）                 → 状态放页面层（page.rs）
```

**卡牌内部独有的部分**（某一张卡才有的展示结构）：

- ✅ 抽成**该 tab 目录下的新组件函数**（如 `tab-page-aliases/list.rs` 里的私有 `fn AliasCardRow(...)`）。
- ❌ **禁止**直接在 `page.rs` 的 rsx 里写这些样式——页面文件一混入样式细节就失去可读性。

### 2.4 舞台模型：多 tab 选择最高高度

卡片的多 tab（内容页签）实现方式是**四态面板叠放**，而不是按内容高度分别撑开：

```
col-start-1 row-start-1                  ← 所有面板占同一格
invisible pointer-events-none            ← 非激活面板隐藏但仍参与布局
```

这样容器高度 = **最高面板的高度**，切换 tab 时卡片不伸缩。修改这条逻辑前必须确认所有已有卡片仍不伸缩。

### 2.5 严禁的写法（反例清单）

| 反例 | 为什么错 | 正确做法 |
|---|---|---|
| `page.rs` 的 rsx 里内联多行 class 写卡牌视觉 | 页面失去「薄组装」性质，样式散落 | 抽到 `tab-page-*/` 的组件，样式封在组件内 |
| 组件内部 `span { "确定" }` 直接写中文 | 违反 H3，无法统一改文案 | 用 `shared.rs` 的 `BTN_CONFIRM` 常量 |
| 组件里 `spawn(async { ApiClient::shared()... })` 直接请求 | 违反交互隔离，组件不可测、不可复用 | 暴露 `EventHandler<()>`，请求留在 `page.rs` |
| 两个 tab 各写一份同样的空态块 | 重复语义 class 串 | 上移 `ui-components::shell::PlaceholderBlock` |
| 把 `border-zinc-800` 顺手改成 `border-border` | 违反 H1（视觉漂移） | 保持原色值，色值迁移单独开 PR |
| 为「将来可能复用」提前抽象 | 过度设计，抽象边界会错 | 出现第 3 个使用方时再抽 |

---

## 3. 文案常量化（i18n 第 1 层）

### 3.1 分层

```
第 1 层（本规范要求）：文案常量化 —— 中文从 rsx 里抽成常量
第 2 层（未来）：多语言切换 —— 常量接入 i18n 表，按 locale 取
```

**当前只做第 1 层，且第 1 层必须零渲染变化**（抽出来的常量值就是原字符串，一个字符都不能改）。

### 3.2 常量命名与放置

常量放**各自 tab 目录的 `shared.rs`**（不要建全局大表，避免越界冲突）：

| 前缀 | 用途 | 示例 |
|---|---|---|
| `LBL_` | 标签 / 表头 / 字段名 | `pub const LBL_ALIAS_NAME: &str = "别名名称";` |
| `BTN_` | 按钮文案 | `pub const BTN_SAVE: &str = "保存";` |
| `SEC_` | 区段标题 / 说明条 | `pub const SEC_LIST_NOTE: &str = "...";` |
| `FIELD_` | 表单字段标签 | `pub const FIELD_MULTIPLIER: &str = "计费倍率 (multiplier ≥ 0)";` |
| `MSG_` | 提示 / 错误 / 空态文案 | `pub const MSG_LOADING: &str = "加载中…";` |
| `OPT_` | 下拉选项文案 | `pub const OPT_PER_TOKEN: &str = "按量";` |

### 3.3 例外（**不抽**的情况）

- **`data-testid` 值不是文案**，保持原样，不抽。
- **纯符号 / 数字 / 单位**（`"¥"`、`"/ 1k tokens"` 里的数字部分、`"—"`）不必抽；但**带中文的单位说明**（`"元/千次"`）要抽。
- **测试文件里的断言字符串**不抽（测试就是要钉死字面值）。
- **`specs/ui/*.yaml` 里的断言**不抽（同理）。

### 3.4 判定与验收

```sh
# 抽完后，页面与组件源码里应基本不再出现中文（注释除外）
grep -rn '"[^"]*[一-龥][^"]*"' crates/web/<crate>/src/tab-page-*/ | grep -v '^\s*//'
```

命中的应当是：常量定义行、注释行。**rsx 元素体内的中文应当为 0**。

---

## 4. 注释

### 4.1 适用范围

- **必须**：所有 `#[component]` 公开组件、所有 `pub` 类型与函数、所有 `use_effect` / `use_signal` 的状态块。
- **建议**：私有组件函数（至少写第 1、2 点）。
- **不适用**：显而易见的 getter、测试函数。

### 4.2 六要素模板

每个 `#[component]` 上方按以下结构写 `///` 文档注释（不需要全填满，但**每题都要回答**，无内容就写「无」）：

```rust
/// 【是什么】一行说清这是个什么组件。
///
/// 【做什么】它负责的功能范围；不负责什么（划清边界）。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互（是否触发网络/改状态）。
/// 无交互时写「纯展示，无交互」。
///
/// 【样式】关键视觉特征：外壳/背景/边框/动效/响应式断点。
///
/// 【子组件组成】内部由哪些组件拼装（列出名字）。
///
/// 【数据流】
/// - 对内（入）：各 prop 的含义与来源。
/// - 对外（出）：EventHandler / Signal 写回会做什么。
#[component]
pub fn AliasCard(...) -> Element { ... }
```

### 4.3 模块级注释

每个 `mod.rs` / 文件头用 `//!` 说明：

- 这个目录/文件**负责什么**；
- 与相邻模块的**边界**（什么逻辑不在这里）；
- 文件名与职责的对应（若做过目录化拆分，说明每个子文件装什么）。

`ui-components/src/components/admin_card/mod.rs` 是范例：每个 `pub mod` / `pub use` 上方都有一句「供管理页展示 XX 的卡片组件」式的说明。

### 4.4 状态块注释

页面 `page.rs` 里成组的 `use_signal` 必须有块注释，说明**这组状态属于谁、为什么放在这一层**：

```rust
// 弹窗表单状态：仅弹窗内部使用，但因为它由「打开弹窗」这一跨组件动作初始化，
// 所以提升到页面层持有，以 Signal<T> prop 传入弹窗。
let mut f_name = use_signal(String::new);
```

---

## 5. 重构阶段（按顺序，每步结束都要能编译）

**每步结束都要能编译**，不许「拆一半留着烂」。卡住或编译不过时，`git reset --hard` 回上一个可编译的提交再继续，不要在烂状态上硬修。

1. **目录化** — 为每个 tab 建 `tab-page-<name>/`；把平铺的 `src/<tab>.rs` 整体移进去，先原样拆成 `page.rs` + `shared.rs`；**此时不做任何样式抽象**，只搬 + 拆文件，保证 `cargo check` 绿；`mod.rs` 里 `pub use page::*;` 保持公开面不变；`lib.rs` 只改路径不改导出名；同步 `crate/README.md` 的文件映射表。
2. **段落解耦** — 在 `page.rs` 里找编号 rsx 段落（`// 1. 统计区` / `// 2. 筛选与操作区` / `// 3. 卡片网格区`），逐段抽成 `stats.rs` / `toolbar.rs` / `list.rs`，按 §2.3 决定状态归属；`page.rs` 缩退为：状态块 + 派生块 + 写回闭包 + 薄 rsx 组装。**每抽一段就编译一次**，并逐行核对 testid / aria / class 未变。
3. **语义样式收敛** — 在整个 `crates/web/` 范围内 grep 重复的语义 class 组合，找出出现 ≥3 次的；收敛进 `ui-components/src/components/admin_card/shell.rs`（或对应域内的 `shell.rs`），导出 `const` + 薄组件。**不改色值**。
4. **卡牌内部抽象** — 找「同一实体卡在不同 tab 里重复实现」的部分（外壳、border 动效、背景、多 tab 叠放高度）；共用部分上移 `ui-components::admin_card`，实体独有的部分留在 `tab-page-*/`；各 tab 的不同内容通过 **slot（`Option<Element>` prop）或独立子组件**填充，禁止用 `#[props]` 传 class 让调用方改样式。
5. **文案常量化**（§3）
6. **注释补齐**（§4）
7. **文档同步** — `crates/web/README.md`（crate 的文件映射与依赖关系）、`crates/web/<crate>/README.md`（目录内文件职责表）、`specs/ui/*.yaml`（若新增/改动了 testid 或 aria 角色，同步断言）。

---

## 6. 单次会话操作流（6 步）

```
1. 选参考     → 打开 .wt/web-visual 的 admin-page-admin，照 tab-page-*/ + lib.rs #[path] 形态
2. 定工作面    → 从仓库根 cd 后 git worktree add .wt/<name> -b feat/<name>；自检路径无第二个 .wt/
3. 保真迁移    → 搬文件 + 拆组件，每步编译一次；公开面只靠 pub use 保留
4. 双重核验    → 符号级 + data-testid 级（脚本见 §7），缺一个都不算完成
5. 命名对齐    → 目录 kebab 化、页面层统一 page.rs、清死导入
6. 验证交付    → clippy -D warnings → gate pre-push → 推 PR → CI 绿
```

⚠️ **接手前先 `git worktree list`**，确认没有别的会话正在改同一 crate（见 §10 坑 2）。

---

## 7. 保真核验（双重，缺一不可）

迁移最危险的**不是编译错，而是悄悄丢 UI**：符号都在、编译绿，但某段渲染没了（数据加载的 signal 还在，渲染没了——死代码，编译不报错）。单靠编译和符号核对发现不了这种回归。

**用 data-testid 做第二重核验**：testid 是渲染 UI 的稳定身份，丢一个 testid 就是丢一块 UI。

### 7.1 符号级

main 平铺文件的每个 `fn` / `struct` / `enum` 在拆分目录里存在（抓改名 / 删除）。

### 7.2 testid 级

```sh
# 核验脚本：main 每个平铺文件的所有 testid，都必须在拆分后的目录里出现
python3 - <<'PY'
import re, glob
main = "crates/web/<crate>/src"            # main 树（或 base_sha 检出）
ref  = ".wt/<name>/crates/web/<crate>/src" # 拆分后
pairs = {"keys":"tab-page-keys", "rewards":"tab-page-rewards"}  # 逐 tab 填
def testids(text):
    s = set()
    for m in re.finditer(r'"data-testid"\s*:\s*"([^"]+)"', text):
        s.add(m.group(1))
    for m in re.finditer(r'"data-testid"\s*:\s*format!\(\s*"([^"]+)', text):
        s.add("FMT:" + m.group(1))
    return s
for flat, sdir in pairs.items():
    ms = testids(open(f"{main}/{flat}.rs").read())
    ss = set()
    for f in glob.glob(f"{ref}/{sdir}/*.rs"):
        ss |= testids(open(f).read())
    missing = sorted(ms - ss)
    print(f"[{flat}] missing={missing if missing else 'none'}")
PY
```

### 7.3 假阴性警告

有些模块（如 account 的 keys / usage_logs）根本没有 testid，脚本输出 `main_testids=0`。此时**不是「核验通过」，而是该模块没有可核验的锚点**——要靠人工 diff 确认渲染段没丢。

---

## 8. 验收清单

提交前逐条自检，全绿才算完成：

- [ ] `cpulimit -l 65 -i -- cargo clippy -p <crate> --all-targets --target wasm32-unknown-unknown -- -D warnings` → 零 warning
- [ ] `cargo fmt --all --check` 通过
- [ ] 符号级核验：main 每个平铺文件的符号在拆分后都存在（§7.1）
- [ ] testid 级核验：main 每个 `data-testid` 在拆分后都存在（§7.2）
- [ ] 公开面逐字不变：`pub use` 的符号名与 main 一致（`apps/admin-web` 与本 crate `tests/` 不需改动）
- [ ] 页面层文件统一 `page.rs`；目录 kebab；`lib.rs` 用 `#[path]` 桥接
- [ ] 页面源码 rsx 内中文为 0（§3.4 的 grep 命令）
- [ ] 所有公开组件具备 6 要素注释
- [ ] crate `README.md` 的文件清单已同步；`specs/ui/*.yaml` 断言未被破坏
- [ ] `gate pre-push` → `RESULT: ALL PASS`，FAIL 清零
- [ ] CI（Formatting / Clippy / Dynamic Crate Check & Test）全绿后再 merge
- [ ] commit message 走 conventional commits；PR body 含任务清单 + base_sha + suspect area（gate 要求英文标题 + 指定 heading，见 `.githooks/spec/github_pull_requests.yaml`）

---

## 9. 范围外（刻意不做，分开开 PR）

- **`ErrCard` 去重**：`tab-page-rewards` 各 section 各定义一份同样的 `ErrCard`。可以抽到 `shared.rs` 或上移 `ui-components`，但涉及 4 个文件的行为合并，留作独立任务，不混在迁移里。
- **色值迁移**：抽组件时保持原色值（`border-zinc-800` 不改 `border-border`），见 §0。
- **文案常量化与目录迁移是不同阶段**，分开做（§5 步骤 1-4 vs 步骤 5）。
- **tavern 侧形态**：dock / layout 结构，不是 tab 组织，`tab-page-*/` 约定不直接适用——先单独定形态再动，不硬套。

---

## 10. 历史踩坑（真实遇到过）

1. **不要碰别人正在开发的 tab**。一个 tab 目录是独占单元，跨 tab 改动要在 PR 里报备。
2. **多 worktree 撞同一个 crate**：曾出现 `.wt/` 下三个分支同时改同一 crate。接手前先 `git worktree list` + 查各分支状态，选一个复用或确认其余已废弃，别开新的。
3. **前序拆分可能丢 UI**：复用已有拆分时，**不要默认它保真**——按 §7 双重核验一遍。曾抓到「最近充值记录」整段列表在拆分中丢失（signal 还在加载，渲染没了）。
4. **死导入是拆分副产物**：组件搬走后，原文件的 import 留在原地变成 unused；clippy 不开 `-D warnings` 看不到，CI 会 FAIL。迁移收尾必须跑一次 `cargo clippy --all-targets -- -D warnings` 清一遍。
5. **bash 工具的 cwd 在调用间不保持**：有两次命令漂移到仓库根，一次让 pre-commit gate 扫到 **main 的 HEAD**（既有 CJK 标题）误报 FAIL，一次 clippy 跑在 main 工作树（验证了错的代码）。**每条命令显式带 `cwd`**；提交/clippy/测试都要确认在目标 worktree 内。
6. **gate FAIL 要看是不是自己的**：CM-02 报的 CJK 标题若来自 main 既有 commit（如 release 提交），与本分支无关，不要去改它，换到 worktree 内重跑即可。
7. **同园但不同源**：`.wt/web-visual` 的 `admin-page-account` **没有**迁过——它的 `tab-page-*` 样板在 `admin-page-admin`。找参考时要逐 crate 确认，别按 worktree 名猜。
8. **删除文件一律 `gio trash`**，禁止 `rm` / `git clean` / `rm -rf`。
9. **合并 main 前先确认 upstream 是 `newxapi/main`**（不是 `origin/main`，后者是 fork 副本）。
10. **本地不跑 `cargo test --all`**（本机内存 <2GB），测试交给 CI。
