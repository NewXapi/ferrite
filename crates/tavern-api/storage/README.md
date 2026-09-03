# `tavern-storage`

## src/lib.rs

- `DataRoot` — 保存酒馆数据根目录。
- `UserDirs` — 生成 `<data>/<user>/characters`、`chats`、`avatars`、`settings.json`、`secrets.json` 路径。
- `sanitize_name` — 拒绝空名、路径分隔符和 `..`。
- `join_checked` — 拼接文件名并确认路径仍在目标目录。
- `write_atomic` — 同目录临时文件写入后 rename。
- `list_by_mtime_desc` — 按修改时间倒序列文件。

## tests/paths.rs

- `路径安全测试` — 覆盖文件名穿越、目录边界、原子覆盖写和用户目录布局。

## 参考实现

- `/home/hathaway/projects/SillyTavern/src/users.js:683` — getUserDirectories。
- `/home/hathaway/projects/SillyTavern/src/util.js:1384` — isPathUnderParent。
