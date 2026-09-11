# tavern-page-personas

用户人格管理。对齐 SillyTavern Persona Management：

- 人格卡牌网格（Web 5 栏 / 平板 3 栏 / 手机 1 栏）
- 每个人格：头像、名字、身份描述、绑定剧本数
- 编辑弹窗：头像占位、名字、描述（表单 3 栏分区 web/平板，1 栏手机）

## 文件

- `src/lib.rs` — 人格结构与页面组件

## 验收

```sh
cargo check -p tavern-page-personas --all-targets
```
