# Ainotation 可视化标注（admin-web）

**给网页 UI 标注反馈、直接交给 AI Agent 修改的工具链。**
浏览器里点选元素 / 框选文字 / 画图截图 → 写修改意见 → Copy/Export 交给 Agent，带 DOM 上下文动手改代码。

来源：[nightire/ainotation](https://github.com/nightire/ainotation)（MIT，beta）。本项目用其 npm beta 包，不改上游源码。

## 三件部件

| 部件 | 位置 | 作用 |
|---|---|---|
| SDK bundle | `apps/admin-web/assets/ainotation/ainotation.iife.js` | 页面内注入标注面板（浮动 **A** 按钮） |
| 前端接线 | `apps/admin-web/src/main.rs` | `#[cfg(debug_assertions)]` 下加载 bundle，release 构建自动排除 |

反馈数据存在**浏览器本地**（IndexedDB，按 projectId `ferrite-admin` + 页面 URL 隔离），无账号、无云端。

## Agent 如何拿到反馈（当前：Copy / Export）

 Ainotation 的 MCP 自动同步依赖 **dev-server 中间件桥**（仅 Vite 插件支持：同源代理 + grant 租约续期，租约仅 5 分钟）。
 Dioxus 的 `dx serve` 无法挂中间件，故当前用官方手动路径，**无桥也能全流程**：

1. 页面标注完成后，点 inspector 工具栏 **Copy feedback**（Markdown）粘贴给 agent；
   或 **Export JSON** 导出文件，把文件路径丢给 agent（含选择器、目标样式、截图 base64，信息量比 Markdown 更全）。
2. Agent 直接读内容/文件改代码，无需任何 MCP 工具。

`ainotation-entry.ts` 保留了运行时读取 `assets/ainotation/connection.json`（gitignore）接 MCP 的钩子：
上游若提供独立 bridge、或页面迁移到 Vite，把合法 grant token 写入该文件即可点亮同步；文件不存在时自动纯本地。
曾验证：MCP 服务与 stdio 协议层可用（`npx --yes @ainotation/mcp@beta connect`，14 个工具），卡点仅在 dx 无桥。

## 日常使用

```bash
# 0. 后端：共享 3211 已在跑则跳过（just dev-check 体检）
# 1. 一条命令起全部（标注栈默认接，幂等复用；详见 .agent/skills/ainotation-web/SKILL.md）
just dev-web 8092          # 共享后端 + 免登录 + 标注栈
#    变体: just dev-web 8092 shared manual (需登录)
#          just dev-web 8092 fresh        (隔离后端 :9092)
#          just dev-web 8092 shared auto off (不接标注)
# 2. 浏览器打开 http://127.0.0.1:8092 ，右下角点浮动 A 按钮
# 3. 标注：
#    - 点击元素 / 拖选文字（按住 Shift 多选）
#    - 按住 Option/Alt 可先操作真实页面（开菜单、填表单）
#    - marker 弹窗里写意见，可加截图 / 画箭头，Cmd/Ctrl+Enter 保存
# 4. 交给 Agent：
#    - agent 直接经 MCP 读（ainotation_get_feedback 等），用户说「看我的标注」即可
#    - 或工具栏 Copy feedback → 粘贴进会话；Export JSON → 告诉 agent 文件路径
```

体检：`just aino-check`（service 注册表 / 桥端点 / 前端连通一次看完）。

### 常用快捷键

| 按键 | 作用 |
|---|---|
| Option/Alt + Shift + A | 开关检查器 |
| 按住 Option/Alt | 操作真实页面 |
| Cmd/Ctrl + Enter | 保存反馈 / 截图 |
| 绘图工具 | V 选 / A 箭头 / R 矩形 / E 椭圆 / F 自由画 / X 裁剪 |
| C / S / D | 换色 / 换线宽 / 删除选中 |

## 改了 SDK 版本或入口后重新打包

```bash
cd apps/admin-web
bun install        # 依赖已入 package.json（@ainotation/sdk）
bun run aino       # 重新生成 assets/ainotation/ainotation.iife.js
```

产物与源 `ainotation-entry.ts` 都已入库；改完提交，其他会话无需重装。

## 边界与已知限制

- **仅开发环境**：release 构建不带 bundle（`debug_assertions` 门控），生产页面无痕。
- **Dioxus 重渲染**：组件结构改动后旧标注的目标节点会被替换，Ainotation 会重新校验并提示目标缺失，**不会**自动绑到新节点——重新标注即可。
- **projectId**：`ferrite-admin`；tavern-web 若接入用独立 projectId（如 `ferrite-tavern`），避免反馈混流。
- **MCP 自动同步**：需 dev-server 桥（见上）；上游仅 Vite 插件自带。SDK 升级后提示不兼容同步时，暂停是预期行为（本地数据保留）。

## tavern-web 复刻（2 步）

1. `apps/tavern-web`：`bun add -d @ainotation/sdk@beta`，拷贝 `ainotation-entry.ts`（改 `projectId: 'ferrite-tavern'`），package.json 加 `aino` 脚本并 `bun run aino`；
2. `main.rs`：拷贝 `#[cfg(debug_assertions)]` 的 asset 常量与 `{ainotation_js.map(...)}` 注入三行。
