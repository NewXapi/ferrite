# tavern-page-lorebook

世界书（Lorebook / World Info）管理。对齐 SillyTavern `#WorldInfo`：

- 世界书卡牌网格（Web 5 栏 / 平板 3 栏 / 手机 1 栏）
- 每本世界书：书名、词条数、常驻/动态策略、最后修改
- 条目编辑弹窗：Web/平板 3 栏分区（关键词/触发设置 | 插入深度/策略 | 设定正文），手机 1 栏堆叠

## 文件

- `src/lib.rs` — 世界书条目结构与页面组件

## 验收

```sh
cargo check -p tavern-page-lorebook --all-targets
```
