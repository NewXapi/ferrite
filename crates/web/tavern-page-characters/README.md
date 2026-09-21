# tavern-page-characters

角色/剧本库与创作中心，内嵌 personas / lorebook 面板。

## 职责

web 域里唯一内嵌其他 page 面板的 crate（`tavern-page-personas` /
`tavern-page-lorebook` 的面板在这里被直接复用）。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/lib.rs` | 角色库 + 创作中心 + 内嵌 personas/lorebook 面板 |

## 硬约束

必须过 `wasm32` check。

## 验收

```bash
cargo check -p tavern-page-characters
cargo check --target wasm32-unknown-unknown -p tavern-page-characters
cargo test -p tavern-page-characters       # CI（data_shapes）
```
