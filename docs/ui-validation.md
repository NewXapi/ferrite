# UI 验证约定

## 目标

agent 验证 dioxus 页面状态时，**不依赖截图**，而是用结构化断言。

## 工具集

OMP 提供 `ui-validate` skill (managed)，封装以下能力：
- `tab.ariaSnapshot()` - 结构化 DOM 快照
- `tab.run()` - 执行 JS 查找 data-testid 元素
- `tab.id("e5").click()` - 操作元素
- `ui-spec.yaml` - 页面契约文件

## 必做项

1. **data-testid 给交互元素**
   - 所有 `button` / `input` / `tab` 加 `data-testid="{name}"` (用 `name` 属性作为 ID)
   - 关键面板/容器加 `role` + `aria-label`

2. **ui-spec.yaml 每个页面一个**
   - 放 `specs/ui/<page>.yaml`
   - 列出关键元素、role、testid、点击动作、期望跳转/状态变化

3. **PR 验证用 ariaSnapshot**
   - 永远先 `tab.ariaSnapshot()`，再考虑截图
   - 断言 role+name+testid，不比对像素

4. **截图只作辅助**
   - 视觉风格/品牌相关才用截图
   - 失败时附图，主验证靠结构化断言

## 示例

```yaml
# specs/ui/admin-overview.yaml
page: "/admin"
name: "管理总览"
elements:
  - role: tab
    name: "用户管理"
    testid: "tab-users"
    on_select:
      - role: table
        testid: "table-users"
        min_rows: 1
```

## 验证流程

agent 在 PR smoke 阶段：

```rust
// 伪代码 - 实际由 ui-validate skill 执行
let snap = tab.ariaSnapshot().await?;
assert!(snap.contains("role=\"heading\" name=\"系统状态\""));

tab.run("document.querySelector('[data-testid=\"tab-users\"]').click()").await?;
let snap2 = tab.ariaSnapshot().await?;
assert!(snap2.contains("role=\"table\" name=\"用户列表\""));
```

## 不要做

- 只用截图肉眼判断（agent 看不清/看不全）
- 用 class 选择器（会因样式调整失效）
- 不写 ui-spec.yaml 直接 PR
