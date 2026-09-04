# `tavern-page-characters`

## `src/lib.rs`

- `CharactersPage`：加载角色列表、选择角色、新建入口。
- `CharacterCard`：名字和描述摘要。
- `CharacterEditor`：`name`、`description`、`personality`、`scenario`、`first_mes`、`mes_example` 表单。
- 保存、删除、进入聊天页回调。

## 验收

```sh
cargo check --target wasm32-unknown-unknown -p tavern-page-characters
```
