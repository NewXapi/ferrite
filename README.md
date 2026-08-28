# New API — web-rs

前端面板（`apps/web` + `crates/web/page`)，Dioxus 0.7 + Tailwind 4。

## 响应式布局约定

页面内容一律放进**同一套栏数的两层网格**（统计带独立一行，面板区独立一行，防止窄面板被自动塞进统计行)，用栏数分三档，写 UI 组件时按这套默认规矩来：

| 断点 | 宽度 | 栏数 |
|------|------|------|
| 手机（默认） | < 768px | 1 栏 |
| 平板（`md:`) | ≥ 768px | 3 栏 |
| Web(`xl:`) | ≥ 1280px | 5 栏 |

每层网格类（见 `crates/web/page/src/overview.rs` 的 `OverviewPanel`):

```
grid grid-cols-1 gap-3 p-4 md:grid-cols-3 md:gap-4 md:p-6 xl:grid-cols-5
```

组件怎么占栏：

- **小统计卡**：占 1 栏（默认，无需类）。手机 1 张/行、平板 3 张/行、Web 5 张/行。
- **宽面板并排**：热力图/图表类宽内容 `md:col-span-2 xl:col-span-3`(3:2 / 2:1 的宽侧），列表/分布类窄内容 `md:col-span-1 xl:col-span-2`。
- **满宽**：确实需要整行的用 `col-span-full`（当前没有）。- **时间范围热力图**（`ActivityGrid`): 全年 3 栏 / 半年 2 栏 / 近3月 1 栏（xl), tab 即 segmented 圆点;格子 `aspect-square` + 周列 `flex-1`, 永不滚动。
- **手机端**全部堆叠；固定最小宽度的内容（如 52 列热力图）包一层 `overflow-x-auto` + `min-w-[Npx]` 横向滚动，不挤压格子。

新面板组件照这套 span 写，不要另起自己的断点体系。

## 开发

```sh
cd apps/web
bun install
dx serve --port 8081   # 热重载
bun run css            # 新增 Tailwind 工具类后重建 assets/tailwind.out.css
```
