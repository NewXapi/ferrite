# `crates/tavern-web`

## 功能 crate

- `client/` — 调用 `/tavern/*`，发送生成请求并消费 SSE。
- `state/` — 当前角色、聊天、消息列表和生成状态。
- `ui/` — 消息、输入、对话框和加载组件。
- `page-characters/` — 角色列表、新建和编辑页面。
- `page-chat/` — 聊天、生成、swipe、编辑和历史页面。
- `page-settings/` — 连接、模型、密钥和采样设置页面。

## MVP：client + state

先开发 `client/` 和 `state/`。二者可以由同一个会话完成。

### `client/src/lib.rs`

- `Character`、`CharacterSummary`、`Message` DTO，字段对齐 `tavern-api/characters/src/lib.rs` 和 `tavern-api/chats/src/lib.rs`。
- `list_characters`、`get_character`、`create_character`、`update_character`、`delete_character`。
- `recent_chats`、`load_chat`、`save_chat`、`delete_chat`。
- `load_settings`、`save_settings`。
- `secrets_state`、`put_secret`。
- `generate`：POST `/tavern/generate`，解析 `data:` SSE 行；读取 `choices[0].delta.content`；`[DONE]` 结束。
- `ApiError`：网络、HTTP 状态和 JSON 解码。

### `state/src/lib.rs`

- `TavernState`：当前角色 file_name、当前聊天名、`Vec<Message>`。
- `GenerationState`：是否生成中、当前累积文本和中止句柄。
- `load_chat`：选角色后读取历史。
- `append_delta`：把 SSE delta 追加到最后一条角色消息。
- `finish_turn`：生成结束后调 client 保存完整聊天。
- `abort`：取消进行中的生成请求。

### 验收

```sh
cargo check --target wasm32-unknown-unknown -p tavern-client -p tavern-state
```

`client/tests/` 覆盖 SSE 的 `data:` 分帧、`[DONE]` 和多段 delta 累积。

## MVP：ui + page-characters

`ui/` 与 `page-characters/` 可和 client/state 并行开发。

### `ui/src/lib.rs`

- `MessageBubble`：名字、内容、用户/角色样式和 Markdown。
- `ChatInput`：多行输入、回车发送、生成中禁用。
- `Dialog`：确认弹窗。
- `Loading`、`EmptyState`。

### `page-characters/src/lib.rs`

- `CharactersPage`：加载列表、选择角色、新建入口。
- `CharacterCard`：名字和描述摘要。
- `CharacterEditor`：`name`、`description`、`personality`、`scenario`、`first_mes`、`mes_example` 表单。
- 保存、删除和进入聊天页回调。

### 验收

```sh
cargo check --target wasm32-unknown-unknown -p tavern-ui -p tavern-page-characters
```

## MVP：page-chat

依赖 client/state。

### `page-chat/src/lib.rs`

- `ChatPage`：按角色加载聊天；无历史时显示角色 `first_mes`。
- `build_messages`：角色卡字段加历史消息，生成 OpenAI `messages`。
- `send`：追加用户消息 → `generate` → `append_delta` → `save_chat`。
- `StreamView`：生成文本和中止按钮。
- `SwipePicker`：切换 `swipes[swipe_id]`。
- `regenerate`：保存当前角色回复到 swipes 后重新生成。
- 单条消息编辑和删除。

### 验收

```sh
cargo check --target wasm32-unknown-unknown -p tavern-page-chat
```

`tests/` 覆盖 `build_messages` 的系统提示和历史顺序。

## MVP：page-settings

依赖 client/state。

### `page-settings/src/lib.rs`

- `SettingsPage`：GET `/tavern/settings` 回填，PUT 保存完整 JSON。
- `ConnectionForm`：模型名和 API key；密钥状态取 `/tavern/secrets`。
- `SamplerForm`：temperature、top_p、max_tokens。
- `ModelsList`：GET `/v1/models`。
- 连通测试：GET `/tavern/status`。

### 验收

```sh
cargo check --target wasm32-unknown-unknown -p tavern-page-settings
```
