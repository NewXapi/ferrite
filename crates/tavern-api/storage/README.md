# `tavern-storage`

## 目录

```text
src/lib.rs
```

## 要实现

- DataRoot 和 UserDirs。
- 角色卡、聊天、头像、设置和密钥目录。
- 原子文件写入。
- 文件名清洗和路径检查。
- 按修改时间列举文件。

## 参考实现

| 能力 | 上游位置 | 机制 |
|------|---------|------|
| 用户目录解析 | `~/projects/SillyTavern/src/users.js:683` `getUserDirectories` | 27 个子目录来自 `src/constants.js:16` `USER_DIRECTORY_TEMPLATE`，按 handle 拼接后进 `DIRECTORIES_CACHE` |
| 原子写 | `~/projects/SillyTavern/src/util.js:1491` `tryWriteFileSync` | 封装 `write-file-atomic`，临时文件加 rename |
| 路径逃逸检查 | `~/projects/SillyTavern/src/util.js:1384` `isPathUnderParent` | 拼接后再校验最终路径仍在父目录内 |
| 首行读取 | `~/projects/SillyTavern/src/util.js:1537` `readFirstLine` | 只读一行做预览，不加载整个 JSONL |
