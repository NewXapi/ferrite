# tavern-presets

单用户预设 JSON 文件管理（对标 SillyTavern `src/endpoints/presets.js`）。每个 API 源（openai、instruct、context、sysprompt、reasoning + kobold/novel/textgenerationwebui）一个子目录，每个预设一个 JSON 文件，落 `<user>/<folder>/<sanitize(name)>.json`。

## 文件

- `src/lib.rs` — 公共结构体与 `http` 模块导出
- `src/http.rs` — `PresetsState` 与 `router`（预设 CRUD 路由）
- `tests/` — 集成测试

## 验收

```sh
cargo check -p tavern-presets --all-targets
```
