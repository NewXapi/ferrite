# `tavern-state`

## `src/lib.rs`

- `TavernState`：全局状态（角色列表、当前角色、聊天、消息、生成状态、用户/模型、错误）。
- `STATE: GlobalSignal<TavernState>`：全局信号，所有页面共享。
- `init`：加载设置 + 角色列表，错误写 `last_error` 不 panic。
- `select_character`：get_character，重置聊天，有 first_mes 时种子首条 assistant。
- `open_chat`：load_chat 填充消息历史。
- `send`：push 用户消息 → 置 generating → build_generate_body → generate → save_chat。
- `abort`：置位后 append_delta no-op。
- `append_delta`：abort 后 no-op；追加到最后一条 assistant 消息。
- `build_generate_body`：PromptInput → render → truncate_history → OpenAI JSON。
- `build_system_prompt`：description + Personality + Scenario + Example dialogue。
- `resolve_swipe`：swipe_id 选中，越界回退 mes。
- `seed_messages`：纯函数，first_mes 种子。

## `tests/state.rs`

无网络，纯逻辑测试：

- build_generate_body 展开 `{{char}}`/`{{user}}` 且带 `_ferrite_agent_prompt_marker`
- 超长历史被 truncate 且首条 system 保留
- swipe_id 选中正确 swipe，越界回退 mes
- select_character 含 first_mes 时种子首条 assistant
- build_system_prompt 拼接非空字段

## 验收

```sh
cargo check --target wasm32-unknown-unknown -p tavern-state
cargo test -p tavern-state
```
