# ferrite 前端开发规范 (纯检测项)

规则只在各 `frontend_*.yaml` 中, 本文件为复刻清单.

```
.githooks/frontend/
├── SPEC_OVERVIEW.md                  # 本文件
├── dispatch.yaml                     # 钩子调度 (gate 接入时启用)
├── frontend_structure.yaml           # FR-01 crate 分层与数据边界
├── frontend_shared_components.yaml   # FR-02 共享组件归属
├── frontend_copy_constants.yaml      # FR-03 文案常量 (反硬编码)
├── frontend_tests.yaml               # FR-04 测试代码划分
└── frontend_no_nested_items.yaml     # FR-05 禁止 fn 内嵌套类型
```

## 5 条硬检查规则

### FR-01 crate 分层与数据边界
- `crates/page-<name>/` (页面) + `crates/{client,session,mock}/` (共享) + `crates/ui/` (组件)
- **FAIL**: 面板直接 `use mock::` (必须经本页 `api.rs` 薄壳)
- **FAIL**: 旧嵌套路径 `crates/page/<n>` 或 `crates/shared/<n>` 存在
- **PASS**: 7 个 page 都有 `src/api.rs` (admin 走 state.rs 除外)

### FR-02 共享组件归属
- ≥2 个 page 用的组件放 `crates/ui/src/<name>.rs`, lib.rs re-export
- **FAIL**: page 内存在 `src/ui.rs`

### FR-03 文案常量 (反硬编码)
- 同一中文字面量复用 **2+ 次** 必须抽 const (组件内或 mod 级)
- **FAIL**: rsx 内出现中文字面量 (api.rs / state.rs / mock 豁免)
- **FAIL**: mock 数据写在面板内 (必须在 crates/mock/)

### FR-04 测试代码划分
- 单元测试放 `src/<file>.rs` 内 `#[cfg(test)] mod tests`
- 集成测试放 `crates/<name>/tests/<topic>.rs`
- **PASS**: 7 个 page 都有 `tests/api_shapes.rs`
- **WARN**: `src/` 内存在独立 `*_test.rs` 或 `test_*.rs`

### FR-05 禁止函数体内定义类型
- **FAIL**: `fn` 体内出现 `struct` 或 `enum` 定义 (必须提到 mod 级或文件顶部)

---

## 明确不做的 (不写规范, 不检查)
- class: 不抽文件、不抽 const、直接写 rsx (改动频繁, 就近维护)
- i18n: 单语言阶段不上 fluent/rust-i18n (中期再考虑)
- rsx 语法: 编译器通过即可, 不做额外检查
- constants crate: 不建独立 crate, 文案按共享范围就近 const
