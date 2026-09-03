# `tavern-page-settings`

## `src/lib.rs`

- `SettingsPage`：GET `/tavern/settings` 回填，PUT 保存完整 JSON。
- `ConnectionForm`：模型名和 API key；密钥状态取 `/tavern/secrets`。
- `SamplerForm`：temperature、top_p、max_tokens。
- `ModelsList`：GET `/v1/models`。
- 连通测试：GET `/tavern/status`。

## `tests/`

- 保存时未知字段保留。
- 密钥输入不回显。

## 验收

```sh
cargo check --target wasm32-unknown-unknown -p tavern-page-settings
cargo test -p tavern-page-settings
```
