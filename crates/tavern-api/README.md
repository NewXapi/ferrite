# `crates/tavern-api`

## 功能 crate

- `storage/` — 用户数据根目录和安全文件读写。
- `auth/` — 酒馆用户身份解析。
- `characters/` — 角色卡 PNG/JSON 管理。
- `chats/` — JSONL 聊天管理。
- `settings/` — 酒馆设置管理。
- `secrets/` — 用户模型密钥管理。
- `presets/` — API 源预设 JSON（OpenAI、instruct、context、sysprompt、reasoning、kobold/novel/textgenerationwebui）。
- `generate/` — 模型生成转发和流输出。
- `media/` — 头像、背景和聊天图片管理。

## MVP：已完成

### `storage/`

- `src/lib.rs` 已有 `DataRoot`、`UserDirs`、文件名清洗、路径检查、原子写和 mtime 列表。
- `tests/paths.rs` 覆盖路径穿越和原子写。

### `auth/`

- `src/lib.rs` 已有 `Identity` 和单机 `default-user` 解析。
- `tests/identity.rs` 覆盖用户目录映射。

### `characters/`

- `src/lib.rs` 已有 Character Card V2 字段、列表和 CRUD。
- `src/png.rs` 已有 PNG `chara` tEXt chunk 的 base64 JSON 读写。
- `src/http.rs` 已有 `/tavern/characters` GET/POST 和单角色 GET/PUT/DELETE。
- `tests/cards.rs`、`tests/png_chunk.rs` 覆盖角色卡和 PNG 读写。

### `chats/`

- `src/lib.rs` 已有 Message、JSONL 覆盖写、容错读取、最近聊天预览、删除和重命名。
- `src/http.rs` 已有 `/tavern/chats/{character}` 和单聊天 GET/PUT/DELETE。
- `tests/jsonl.rs` 覆盖 JSONL、未知字段、坏行和路径穿越。

### `settings/`

- `src/lib.rs` 已有 settings.json 读写和未知字段保留。
- `src/http.rs` 已有 `/tavern/settings` GET/PUT。

### `secrets/`

- `src/lib.rs` 已有密钥读写删除和不带明文的配置状态。
- `src/http.rs` 已有 `/tavern/secrets` GET 和单 key PUT/DELETE。
- `tests/masking.rs` 覆盖明文不回显。

### `presets/`

- `src/lib.rs` 已有 API 源到子目录的映射（openai/instruct/context/sysprompt/reasoning + kobold/novel/textgenerationwebui）和 save/load/list/delete/restore。
- `src/http.rs` 已有 `/tavern/presets` 列表/保存/删除（同一 `/`，三方法）和 `/restore`。
- `tests/presets.rs` 覆盖保存/读取/列表/删除/未知 apiId/路径穿越/空 restore/原子覆盖写。

### `generate/`

- `src/lib.rs` 已有 `/tavern/generate` POST：读取当前用户 key、转发 OpenAI 请求体、透传上游响应流。
- `src/lib.rs` 已有 `/tavern/status` GET。

### `apps/api/src/tavern.rs`

- 已组装 `/tavern/*` 的 characters、chats、settings、secrets 和 generate 路由。
- `apps/api/tests/tavern.rs` 覆盖角色、聊天、设置、密钥和路径穿越的真实 HTTP 往返。

## MVP：media

`media/` 是酒馆 API 下一项独立开发。

### `media/src/lib.rs`

- `AvatarStore`：`<user>/avatars` 上传、列举和删除。
- `ChatImageStore`：`<user>/images` 聊天图片上传、读取和删除。
- `BackgroundStore`：`<user>/backgrounds` 背景图上传、列举和删除。
- 上传文件类型白名单：PNG、JPEG、WebP、GIF。
- 上传大小限制。

### `media/src/http.rs`

- `GET/POST/DELETE /tavern/avatars`。
- `GET/POST/DELETE /tavern/images`。
- `GET/POST/DELETE /tavern/backgrounds`。

### `media/tests/`

- 上传类型和大小校验。
- 文件名路径穿越。
- 上传、列举、读取和删除。

### 验收

```sh
cargo test -p tavern-media
```
