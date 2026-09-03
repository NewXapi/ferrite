# Ferrite

单机 API 聚合：中转网关 + 管理后台 + 酒馆。`crates/<域>/` 是聚合目录，其下每个子目录是一个 lib crate；进程入口只在 `apps/`。

```text
apps/
├── api/                 # 后端进程：gateway + admin-api + tavern-api + harness runtime
├── web/                 # 管理后台前端
└── tavern-web/          # 酒馆前端

crates/
├── contract/            # 跨域共享类型
├── gateway/
│   ├── pipeline/
│   ├── gate/
│   ├── dispatch/
│   ├── forward/
│   ├── proxy/
│   ├── protocol/
│   ├── protocol-bridge/
│   ├── metering/
│   └── security/
├── admin-api/
│   ├── store/
│   ├── sync/
│   ├── catalog/
│   ├── billing/
│   ├── observe/
│   └── ops/
├── admin-web/
│   ├── client/
│   ├── session/
│   ├── ui/
│   ├── mock/
│   ├── page-auth/
│   ├── page-overview/
│   ├── page-account/
│   ├── page-admin/
│   └── page-users/
├── tavern-api/
│   ├── storage/
│   ├── auth/
│   ├── characters/
│   ├── chats/
│   ├── settings/
│   ├── secrets/
│   ├── generate/
│   └── media/
├── tavern-web/
│   ├── client/
│   ├── state/
│   ├── ui/
│   ├── page-characters/
│   ├── page-chat/
│   └── page-settings/
└── harness/
    ├── core/
    ├── prompt/
    ├── tools/
    ├── runtime/
    └── ui/
```

各域要实现的功能见对应目录的 `README.md`。

## 路由

```text
admin-web  ──→  /admin/*     管理
tavern-web ──→  /tavern/*    酒馆
客户端      ──→  /v1/*        OpenAI 兼容中转
```

## 管理前端布局

页面内容放进同一套栏数的两层网格（统计带独立一行，面板区独立一行），用栏数分三档：

| 断点 | 宽度 | 栏数 |
|------|------|------|
| 手机（默认） | < 768px | 1 栏 |
| 平板（`md:`） | ≥ 768px | 3 栏 |
| Web（`xl:`） | ≥ 1280px | 5 栏 |

```
grid grid-cols-1 gap-3 p-4 md:grid-cols-3 md:gap-4 md:p-6 xl:grid-cols-5
```

- **小统计卡**：占 1 栏。
- **宽面板并排**：热力图/图表 `md:col-span-2 xl:col-span-3`，列表/分布 `md:col-span-1 xl:col-span-2`。
- **满宽**：`col-span-full`。
- **手机端**全部堆叠；固定最小宽度的内容包 `overflow-x-auto` + `min-w-[Npx]`。

## 开发

```sh
cargo run -p api                    # 后端

cd apps/admin-web && bun install
dx serve --port 8081                # 管理前端
bun run css
```
