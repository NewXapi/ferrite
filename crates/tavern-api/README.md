# `crates/tavern-api`

## 功能 crate

- `storage/` — 用户数据根目录和安全文件读写。
- `auth/` — 酒馆用户身份解析。
- `characters/` — 角色卡 PNG/JSON 管理。
- `chats/` — JSONL 聊天管理。
- `settings/` — 酒馆设置管理。
- `secrets/` — 用户模型密钥管理。
- `generate/` — 模型生成转发和流输出。
- `media/` — 头像、背景和聊天图片管理。

## MVP 调用链

- `storage → auth` — 根据当前用户取得角色、聊天、设置和密钥路径。
- `characters + chats + settings + secrets` — 提供 `/tavern/*` REST 路由。
- `generate` — 把聊天页发送的 OpenAI 请求转发到 `/v1/chat/completions`。

