# `contract`

## 目录

```text
contract/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── api/
    ├── records/
    ├── mutations.rs
    ├── error.rs
    └── schema.rs
```

## 要实现

- 定义管理 API DTO、响应信封和错误码。
- 定义渠道、路由、身份、计费和用量记录。
- 定义配置同步 mutation、cursor、版本摘要和确认消息。
- 定义 JSON schema 与兼容 fixture。
