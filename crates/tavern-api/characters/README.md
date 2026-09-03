# `tavern-characters`

## src/lib.rs

- `Character` — Character Card V2 基础字段和未知字段透传。
- `CharacterSummary` — 角色列表展示字段。
- `get/save/delete/rename/list` — 角色卡文件 CRUD。

## src/png.rs

- `read_chara` — 读取 PNG `chara` tEXt chunk 内的 base64 JSON。
- `write_chara` — 写入或替换 PNG `chara` tEXt chunk。
- `minimal_png` — 新建角色的默认 1×1 PNG 底图。

## src/http.rs

- `router` — `GET/POST /tavern/characters` 与 `GET/PUT/DELETE /tavern/characters/{name}`。
- `CharactersState` — 当前用户 UserDirs。

## tests/cards.rs

- `角色卡 CRUD 测试` — 保存、读取、编辑、列表容错、删除、路径穿越。

## tests/png_chunk.rs

- `PNG chunk 测试` — 写读、重复写覆盖、不同长度 JSON、非法 PNG。

## 参考实现

- `/home/hathaway/projects/SillyTavern/src/endpoints/characters.js:220` — writeCharacterData。
- `/home/hathaway/projects/SillyTavern/src/character-card-parser.js:15` — PNG tEXt write。
