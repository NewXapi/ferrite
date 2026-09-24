# admin-catalog

渠道 / 模型 / 分组 / Token 的管理面 CRUD。

## 职责

平表直连的后台管理接口，供 admin 页面的 channels / models / groups / tokens 四个
tab 消费。DTO 的唯一事实来源在本 crate 的 `*View` 结构体，改后端序列化必须同步
`contract`，并先用真实抓包 JSON 写回归测试再改实现。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/channels.rs` | 渠道 CRUD |
| `src/models.rs` | 模型 CRUD |
| `src/groups.rs` | 分组 CRUD |
| `src/tokens.rs` | API Token CRUD（明文只出现一次） |
| `src/lib.rs` | crate 导出面 |

## 验收

```bash
cargo check -p admin-catalog
cargo test -p admin-catalog                # CI
```
