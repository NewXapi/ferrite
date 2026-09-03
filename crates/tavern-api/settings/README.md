# `tavern-settings`

## 目录

```text
src/
├── lib.rs
└── http.rs
```

## 要实现

- settings.json 读取和保存。
- 连接配置、采样参数和界面设置。
- 设置快照和预设。

## 参考实现

| 能力 | 上游位置 | 机制 |
|------|---------|------|
| 读存 | `~/projects/SillyTavern/src/endpoints/settings.js:206` `/save` `:219` `/get` | `/get` 同时返回设置与各类预设清单 |
| 快照 | `~/projects/SillyTavern/src/endpoints/settings.js:298` | get-snapshots / make-snapshot / load-snapshot / restore-snapshot |
