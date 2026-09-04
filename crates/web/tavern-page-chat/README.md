# `tavern-page-chat`

## `src/lib.rs`

- `ChatPage`：按角色加载聊天；无历史时显示角色 `first_mes`。
- `build_messages`：角色卡字段加历史消息，生成 OpenAI `messages`。
- `send`：追加用户消息 → `generate` → `append_delta` → `save_chat`。
- `StreamView`：生成文本和中止按钮。
- `SwipePicker`：切换 `swipes[swipe_id]`。
- `regenerate`：保存当前角色回复到 swipes 后重新生成。
- 单条消息编辑和删除。

## `tests/`

- `build_messages` 系统提示。
- 历史消息顺序。
- swipe_id 更新。

## 验收

```sh
cargo check --target wasm32-unknown-unknown -p tavern-page-chat
cargo test -p tavern-page-chat
```
