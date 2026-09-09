# Ferrite

单机 API 聚合：OpenAI 兼容中转网关 + 管理后台 + 酒馆前端。Rust workspace，后端 Axum，前端 Dioxus（编译到 `wasm32-unknown-unknown`）。

`crates/` 下按大域平铺，每个子目录是一个 lib crate；进程入口只在 `apps/`。

```text
apps/                    # 进程入口，只做配置、状态与路由组装
├── api/                 # 后端单体：admin-api + tavern-api + harness runtime
├── gateway/             # 网关数据面进程
├── admin-web/           # 管理后台前端
└── tavern-web/          # 酒馆前端

crates/
├── contract/            # 跨端共享 DTO 与协议错误，前后端唯一共享点
├── api/                 # 后端服务
│   ├── auth/            #   平台账号中心（Argon2id + JWT）
│   ├── admin-*/         #   store / sync / catalog / billing / observe / ops / router
│   └── tavern-*/        #   storage / auth / characters / chats / settings /
│                        #   presets / secrets / generate / media
├── web/                 # 前端组件与页面
│   ├── ui-components/   #   跨端通用组件
│   ├── admin-client/    #   admin 侧 API client / session / mock
│   ├── admin-page-*/    #   auth / overview / account / admin / users
│   ├── tavern-client/   #   tavern 侧 API client / state
│   └── tavern-page-*/   #   home / characters / chat / personas / lorebook / settings
├── gateway/             # 网关数据面
│   ├── pipeline/        #   Stage trait 与请求上下文
│   ├── gate/            #   准入与配额闸门
│   ├── dispatch/        #   渠道选择：健康度 + 加权随机 + failover
│   ├── forward/         #   上游转发与 SSE 流式透传
│   ├── protocol-bridge/ #   厂商协议适配（OpenAI / Claude / Gemini）
│   ├── metering/        #   token 计量与结算
│   ├── proxy/           #   出站代理拨号
│   └── security/        #   内容安全与敏感词
└── harness/             # Agent 运行时
    ├── core/            #   run 状态机
    ├── prompt/          #   prompt 组装与截断
    ├── tools/           #   工具调用
    ├── runtime/         #   事件循环与持久化
    └── ui/              #   运行时 UI

tests/                   # 跨 crate 端到端集成测试
scripts/                 # CI 动态选包等工程脚本
config/                  # 运行配置（config.toml 不入库）
```

各域要实现的功能见对应目录的 `README.md`。

## 约束

- 域间禁止直接依赖：跨域引用只允许经 `crates/contract`。
- `crates/web/*`、`apps/{admin-web,tavern-web}`、`harness/{core,prompt,tools}` 必须能编译到 `wasm32-unknown-unknown`。
- 测试放同层 `tests/`，不在 `src/` 里写 `#[cfg(test)]`。

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

首次 clone 后先建本地配置（不入库）：

```sh
cp config/config.toml.example config/config.toml
```

起服务：

```sh
cargo run -p api                     # 后端（bin 名 ferrite）
cargo run -p gateway                 # 网关数据面

cd apps/admin-web  && bun install && dx serve --port 8081   # 管理前端
cd apps/tavern-web && bun install && dx serve               # 酒馆前端
bun run css                          # Tailwind watch
```

## 验证

本地只做类型检查，测试交给 CI —— 全量 workspace 测试会吃掉开发机内存。

```sh
cargo check -p <crate>                                          # 后端
cargo check --target wasm32-unknown-unknown -p <crate>          # 前端
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
```

clippy 版本要与 CI 一致（CI 用 `dtolnay/rust-toolchain@stable`），否则本地跑绿 CI 仍可能报新 lint：

```sh
rustup update stable
```

## CI

PR 只跑受改动影响的包，合并到 main 跑全量兜底。

| 触发 | 行为 |
|---|---|
| PR | `scripts/ci-affected.sh` 按 `git diff` 选包 + 反向依赖闭包 |
| push main | `cargo build --all-targets` + `cargo test --all` |
| 两者 | `cargo fmt --all --check`、`cargo clippy --all-targets -- -D warnings` |

选包逻辑：路径命中得到直接改动的包，再沿 workspace 内部依赖图（`cargo metadata` 的 `path` 依赖）反向 BFS，补齐所有依赖它的下游包。改 `Cargo.toml` / `Cargo.lock` / `rust-toolchain.toml` / `.github/*` / `scripts/*` 升级为全量。

本地预览选包结果：

```sh
bash scripts/ci-affected.sh --base newxapi/main --dry-run
```
