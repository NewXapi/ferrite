# `crates/tavern-api`

## 目录

```text
tavern-api/
├── storage/
├── auth/
├── characters/
├── chats/
├── settings/
├── secrets/
├── generate/
└── media/
```

## 要实现

- `storage` 提供数据根目录、用户目录、原子写和安全路径。
- `auth` 解析酒馆用户身份。
- `characters` 管理角色卡和角色头像。
- `chats` 管理 JSONL 聊天记录。
- `settings` 保存用户连接、采样和界面设置。
- `secrets` 保存用户上游密钥并提供已配置状态。
- `generate` 转发生成请求、透传 SSE、支持中止。
- `media` 管理头像、背景和聊天图片。
