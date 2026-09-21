# web — 前端

Dioxus wasm 前端 crate 群。分管理端与酒馆端，加一个跨端共享组件层。

## crate 清单

| crate | 职责 |
|---|---|
| `ui-components` | 跨端通用组件层，**只依赖 contract，不依赖任何 client/page** |
| `admin-client` | 管理 API 客户端——Bearer 注入、`Envelope<T>` 解码、401 回调 refresher |
| `admin-page-auth` | 登录/注册认证页（state context + 公开入口） |
| `admin-page-overview` | 总览/模型/排行榜三个 tab（`tab-page-*/` 目录化） |
| `admin-page-account` | 个人中心——密钥、用量日志、会话、奖励、设置（tab 目录化） |
| `admin-page-admin` | 管理操作——渠道/分组/别名/兑换/网络/系统等 10+ 个 tab（`tab-page-*/` 目录化） |
| `admin-page-users` | 用户管理面板 |
| `tavern-client` | `/tavern/*` 请求 + `generate` SSE 分帧解析 |
| `tavern-state` | 角色/聊天/消息全局状态 + 生成中状态 + dock 布局状态 |
| `tavern-page-home` | 品牌落地页 |
| `tavern-page-characters` | 角色/剧本库、创作中心（内嵌 personas/lorebook 面板） |
| `tavern-page-chat` | 聊天与流式生成互动界面（dock 布局） |
| `tavern-page-personas` | 用户人格管理 |
| `tavern-page-lorebook` | 世界书管理 |
| `tavern-page-settings` | 连接与采样设置 |

## 硬约束

- **全部 web crate 必须支持 `wasm32-unknown-unknown`**：
  `cargo check --target wasm32-unknown-unknown -p <crate>`
- page crate 互相不感知（唯一例外：`tavern-page-characters` 内嵌 personas/lorebook 面板）
- 组装只发生在 `apps/admin-web` / `apps/tavern-web`
- 测试放同层 `tests/`，**不得**在 `src/` 里写 `#[cfg(test)]`
- 交互元素加 `data-testid`（值取 `name` 属性），容器加 `role` + `aria-label`；
  PR 冒烟用 `tab.ariaSnapshot()` 做结构化断言（`.agent/skills/ui-validation/`）

## 依赖与越界边（实测）

- `ui-components` / `admin-client` / `tavern-client` 只依赖 `contract`
- **web → harness 唯一越界边**：`tavern-state → harness/prompt`
- `Cargo.toml` 改名依赖：`ui` = `ui-components`、`client` = `admin-client`、
  `tavern_client` / `tavern_state`、`page-auth` / `page-account` / `page-overview` /
  `page-admin` / `page-users` = `admin-page-*`

## 验收

```bash
cargo check --target wasm32-unknown-unknown \
  -p ui-components -p admin-client -p admin-page-auth -p admin-page-overview \
  -p admin-page-account -p admin-page-admin -p admin-page-users \
  -p tavern-client -p tavern-state -p tavern-page-home -p tavern-page-characters \
  -p tavern-page-chat -p tavern-page-personas -p tavern-page-lorebook \
  -p tavern-page-settings
```
