# `tavern-settings`

## src/lib.rs

- `load` — 读取 settings.json；文件不存在返回空对象。
- `save` — 原子保存完整设置 JSON，未知字段保留。

## src/http.rs

- `router` — `GET/PUT /tavern/settings`。
- `SettingsState` — 当前用户 UserDirs。

## tests/roundtrip.rs

- `设置测试` — 空设置与未知字段往返。

