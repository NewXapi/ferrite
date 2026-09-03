# `tavern-secrets`

## 目录

```text
src/
├── lib.rs
└── http.rs
```

## 要实现

- secrets.json 密钥存取。
- 密钥已配置状态查询。
- 静态加密。
- 多组密钥和标签。

## 参考实现

| 能力 | 上游位置 | 机制 |
|------|---------|------|
| 键名表 | `~/projects/SillyTavern/src/endpoints/secrets.js:9` `SECRET_KEYS` | 按上游厂商固定键名 |
| 读写删 | `~/projects/SillyTavern/src/endpoints/secrets.js:428` `writeSecret` `:437` `deleteSecret` `:448` `readSecret` | `readSecret` 只在服务端调用 |
| 对外状态 | `~/projects/SillyTavern/src/endpoints/secrets.js:457` `readSecretState` | 返回是否已配置；明文回显由 `:108` `allowKeysExposure` 单独开关控制 |
