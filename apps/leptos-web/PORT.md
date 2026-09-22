# leptos 页面移植

工作目录 `/home/hathaway/projects/ferrite/.wt/leptos-web`。
只改自己那一个 `apps/leptos-web/src/pages/<name>.rs`。
不要改 `app.rs`、`lib.rs`、`main.rs`、`ui.rs`、`Cargo.toml`、`pages/mod.rs`。
不要 `cargo build` / `cargo test` / `cargo check`。不要 commit。

## 组件

先用 singlestage，不要自己画按钮、卡片、弹窗、下拉、分页。

```rust
use leptos::prelude::*;
use singlestage::*;
use crate::ui::CardGrid;
```

- `Button`：`variant` 取 `primary` / `secondary` / `outline` / `ghost` / `destructive`，`size` 取 `small` / `icon`。页面按钮写 `button_type="button"`，否则默认是 submit。
- `Card` + `CardHeader` + `CardTitle` + `CardContent` + `CardFooter`
- `Badge`、`Input`、`Dialog`、`Select`、`Tabs`、`Table`、`Pagination`、`Separator`
- 卡片列表包 `<CardGrid>`。web 5 栏，平板 3 栏，手机 1 栏。不要自己再写 grid。

singlestage 的 class 是 `class=`，不是 dioxus 的 `class:`。
`Dialog` 需要 `dialog_trigger=DialogTrigger` 和一个 `open` 信号。
`Tabs` 用 `value` 绑定当前页签。`Select` 用 `value` 绑定当前值。

## 数据和交互

界面照 dioxus 源码搬，class 逐字搬。
数据用源码里的静态/演示数据。筛选、tab、弹窗、启停都用 `RwSignal`，点了就变，不刷新页面。
不要接真实 API，不要加 server function，不要加依赖。
