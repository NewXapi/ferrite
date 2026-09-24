# ui-components

跨端通用组件层。**只依赖 `contract`，不依赖任何 client/page。**

## 职责

管理端与酒馆端共用的 Dioxus 组件。`src/components/` 按「一个目录一个组件族」组织
（`mod.rs` 只做 re-export，实现在 `component.rs`）。

## 文件清单

### 组件族（`src/components/`）

| 目录 | 组件 |
|---|---|
| `admin_card/` | 管理区卡牌族：`AdminCard` 四态面板 + `CardShell` 语义壳 + `Pager` 分页 + `PriceMode` + aliases/channels/users 实体卡 + `editable` 行内编辑 + `dot_tab` |
| `avatar/` `badge/` `button/` `card/` `input/` `select/` `switch/` `sheet/` `toast/` `sidebar/` `skeleton/` `stat_card/` `dropdown_menu/` `rank_board/` | 基础组件（`mod.rs` + `component.rs`） |
| `layout/` | `app_shell` / `section_rail` / `top_nav` / `status_bar` |
| `showcase/` | `poster_card` / `radar_flip_card` / `stat_tabs_card` |

### 平铺模块（`src/`）

| 文件 | 职责 |
|---|---|
| `src/lib.rs` | crate 导出面 |
| `src/dialog.rs` | 对话框 |
| `src/form.rs` | 表单 |
| `src/feedback.rs` | 反馈 |
| `src/bubble.rs` | 气泡 |
| `src/segmented.rs` | 分段控件 |
| `src/session.rs` | 会话 |
| `src/i18n.rs` | 国际化文案表 |
| `src/icons.rs` | 图标 |
| `src/scroll_spy.rs` | 滚动联动 |
| `src/wheel_tab.rs` | 滚轮 tab 切换 |
| `src/action_buttons.rs` | 操作按钮组 |
| `src/auth_modal.rs` | 登录弹窗 |
| `src/components/mod.rs` | 组件目录 re-export |

## 硬约束

必须过 `wasm32-unknown-unknown` check；不依赖任何 client/page crate。

## 验收

```bash
cargo check -p ui-components
cargo check --target wasm32-unknown-unknown -p ui-components
cargo test -p ui-components                # CI（p2a/p2b/p2c/p2d_contract 锁定组件契约）
```
