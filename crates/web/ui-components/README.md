# ui-components

全栈通用的 Web 组件库（Dioxus，wasm32 目标）。整合原 admin-ui 与 tavern-ui，按文件拆分：

- `auth_modal.rs` — 通用认证弹窗与用户状态微标（AuthModal, UserBadge）
- `session.rs` — 会话凭证管理与登录注册客户端
- `form.rs` — 表单基元（Field, CodeField, SubmitButton, SliderField）
- `feedback.rs` — 头像、图标按钮、空态、加载指示器（Avatar, IconButton, EmptyState, Loading）
- `bubble.rs` — 对话气泡与分支切换器（MessageBubble, SwipePicker）
- `card.rs` — 状态与行动决策卡片（StatusCard, ChoiceCard, ChoiceOption）
- `dialog.rs` — 确认弹窗（Dialog）
- `scroll_spy.rs` — 滚动监听导航（ScrollSpyNav）
- `segmented.rs` — 分段胶囊选择器（SegmentedCapsule）
- `components/` — 自研对齐 shadcn 的原语（button/input/badge/dropdown/sidebar/sheet/toast/select/switch/avatar/skeleton）

## 约定

- 跨端数据交互必须走 `crates/contract` DTO，不直接依赖 API crate
- 组件加 `data-testid`（用 `name` 属性值），容器加 `role` + `aria-label`，配合 `specs/ui/` 契约

## 验收

```sh
cargo check -p ui-components --all-targets
cargo check -p ui-components --target wasm32-unknown-unknown   # wasm 目标
```
