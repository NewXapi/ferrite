# `tavern-characters`

## 目录

```text
src/
├── lib.rs
├── http.rs
└── png.rs
```

## 要实现

- Character Card V2 数据结构。
- PNG `chara` tEXt chunk 读写。
- 角色创建、读取、编辑、删除、重命名和列表。
- PNG 与 JSON 导入导出。
- 角色头像上传。

## 参考实现

| 能力 | 上游位置 | 机制 |
|------|---------|------|
| 角色卡写盘 | `~/projects/SillyTavern/src/endpoints/characters.js:220` `writeCharacterData` | 角色 JSON 写进 PNG tEXt 后原子落盘 |
| PNG tEXt 读写 | `~/projects/SillyTavern/src/character-card-parser.js:15` `write` / `:54` `read` | `ccv3` 与 `chara` 两个关键字，base64 编码 |
| 卡规范校验 | `~/projects/SillyTavern/src/validator/TavernCardValidator.js:8` | V1 / V2 / V3 三种 spec 分别校验 |
| 列表浅解析 | `~/projects/SillyTavern/src/endpoints/characters.js` `processCharacter` + `DiskCache` | 列表只取展示字段，避免全量解析每张卡 |
