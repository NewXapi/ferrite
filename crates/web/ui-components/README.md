# ui-components

跨端通用组件层。**只依赖 `contract`，不依赖任何 client/page。**

## 职责

管理端与酒馆端共用的 Dioxus 组件。按「组件类型」聚合：一个目录一个类型族，
目录内**一个组件一个文件**；单一组件直接平铺为 `src/<类型>.rs`，不设单文件目录。

## 文件清单

### 类型族目录（`src/`）

| 目录 | 文件（一组件一文件） | 组件 |
|---|---|---|
| `button/` | `button` `icon_button` `submit_button` `action_button` `ghost_button` `close_button` | 全部按钮型：`Button`(+Variant/Size)、`IconButton`、`SubmitButton`、`ActionButton(Group/Spec/Tone)`、`GhostButton`、`CloseButton` |
| `card/` | `card` `status_card` `choice_card` `stat_card` | shadcn `Card` 七件套 + `StatusCard` / `ChoiceCard(+Option)` / `StatCard(+StatSize)` |
| `form/` | `field` `form_field` `code_field` `slider_field` `password_field` | 表单基元；共享 `INPUT_CLASS` 在 `mod.rs` |
| `feedback/` | `avatar` `empty_state` `loading` | 兼容包装 `Avatar`（默认 36px 档，委托 `avatar::Avatar`）、`EmptyState`、`Loading` |
| `auth/` | `auth_modal` `user_badge` | `AuthModal`、`UserBadge` |
| `bubble/` | `message_bubble` `swipe_picker` | `MessageBubble`、`SwipePicker` |
| `icons/` | 一图标一文件（19 个） | lucide 风格单色 stroke 图标集 |
| `admin_card/` | `card` `shell` `aliases` `channels` `users` `pager` `editable` `price_mode` `dot_tab` | 管理区卡牌族：`AdminCard`、实体卡、`CardShell`、`Pager`、`EditableRow`、`PriceMode`、`DotTabBar` |
| `layout/` | `app_shell` `section_rail` `top_nav` `status_bar` `avatar_menu` | 布局原语 |
| `showcase/` | `poster_card` `radar_flip_card` `stat_tabs_card` | 展示卡牌 |

### 单文件类型组件（`src/<类型>.rs`）

`dialog.rs`(Dialog)、`panel.rs`(FieldPanel + MODAL_* 样式常量)、`nav.rs`(ScrollSpyNav)、
`segmented.rs`(SegmentedCapsule)、`sheet.rs`、`sidebar.rs`、`dropdown_menu.rs`、
`toast.rs`、`rank_board.rs`、`badge.rs`、`avatar.rs`(shadcn 基元)、`input.rs`、
`skeleton.rs`、`switch.rs`、`select.rs`(+state_parts 契约纯函数)

### 平铺模块（非组件）

`src/lib.rs` 导出面 · `src/session.rs` 会话 · `src/i18n.rs` 文案 ·
`src/styles.rs` 样式 token · `src/wheel_tab.rs` 滚轮 tab 切换

### 过渡层（待删）

`src/components/` — rust-ui registry 拷贝线（`rui_*` 前缀，shadcn copy-paste）。
替换计划见 `todo/rust-ui-keys-replace-plan.md`，全部替换完成后整目录删除。

## 硬约束

必须过 `wasm32-unknown-unknown` check；不依赖任何 client/page crate。

## 验收

```bash
cargo check -p ui-components
cargo check --target wasm32-unknown-unknown -p ui-components
cargo test -p ui-components                # CI（p2a/p2b/p2c/p2d_contract 锁定组件契约）
```
