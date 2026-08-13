# nginx + web — 入口与前端静态资源

两者放一份文档，因为 web 不是进程，就是 nginx 托管的一卷静态文件。

---

# nginx

- 镜像：`nginx:alpine`
- 监听：`:80` / `:443`
- 副本：1+（无状态，可任意扩）
- 今天的等价物：gin 自己兼任静态文件服务（`router/web-router.go:28` `static.Serve`）+ k8s ingress

## 1. 内部功能

### 1.1 TLS 卸载

- 证书挂载，443 → 内部明文
- gateway 与 control 都只听 HTTP

### 1.2 路径分发（7 类前缀）

发给 **gateway**：
- `/v1/` — OpenAI 兼容主入口
- `/v1beta/` — Gemini 原生
- `/mj/`、`/:mode/mj/` — Midjourney
- `/suno/` — Suno
- `/kling/v1/` — Kling
- `/jimeng` — 即梦
- `/pg/` — Playground

发给 **control**：
- `/api/`
- `/dashboard/`、`/v1/dashboard/` — OpenAI 兼容用量查询（注意这条与 `/v1/` 前缀重叠，
  必须用更长前缀优先匹配，`location = /v1/dashboard/billing/usage` 之类的精确规则要排在 `/v1/` 之前）

发给 **web**（静态文件）：
- `/` 其余全部
- SPA 兜底：找不到文件返回 `index.html`（对应今天 `router/web-router.go:29-37` 的 `NoRoute` 逻辑）

### 1.3 必须的配置项

否则 SSE 直接坏：
- `proxy_buffering off` — 关闭响应缓冲
- `proxy_request_buffering off` — 关闭请求缓冲
- `proxy_read_timeout 3600` — 长流式响应
- `proxy_send_timeout 3600`
- `client_max_body_size 0` — 大请求体（多模态附件），改由应用层 `MAX_REQUEST_BODY_MB` 控制
- WebSocket 升级头透传 — `/v1/realtime` 用（`router/relay-router.go:78`）

依据：`deploy/k8s/ingress.yaml:17-23` 已经有等价的 annotation，注释里也写明了
"流式（SSE）响应必须关闭缓冲，否则客户端收不到增量输出"。

### 1.4 静态资源缓存

- `/assets/*` — 长 `Cache-Control`（构建产物带 hash）
- `index.html` — `no-cache`（对应今天 `router/web-router.go:35` `c.Header("Cache-Control", "no-cache")`）
- gzip — 今天在 gin 层做（`router/web-router.go:25`、`api-router.go:18`），移到 nginx

## 2. 数据流动

nginx 本身不产生数据流，只做转发。它参与的流见 `07-inter-app-dataflow.md` 的 F1（推理）、
F2（管理）、F3（前端资源）、F5（支付回调入口）。

一个要注意的点：**nginx 是 gateway 与 control 的唯一入口**，
所以真实客户端 IP 必须靠 `X-Forwarded-For` 传递。
今天有 `middleware/trusted_proxies.go` `ConfigureTrustedProxies`（`main.go:177` 调用），
Rust 侧要保留等价逻辑，否则 IP 限流（`middleware/rate-limit.go:44` `redisIPRateLimitKey`）会全部命中 nginx 的 IP。

## 3. 目录结构

```
deploy/nginx/
├── Dockerfile                    # FROM nginx:alpine, COPY dist + conf
├── new-api.conf                  # 主配置
│   ├── upstream gateway          # gateway:8080
│   ├── upstream control          # control:8081
│   ├── location /v1/dashboard/   # ← 必须排在 /v1/ 之前
│   ├── location /v1/             # → gateway, 关缓冲, 3600s
│   ├── location /v1beta/         # → gateway
│   ├── location /mj/ /suno/      # → gateway
│   ├── location /kling/ /jimeng  # → gateway
│   ├── location /pg/             # → gateway
│   ├── location /api/            # → control
│   ├── location /dashboard/      # → control
│   ├── location /assets/         # → 静态, 长缓存
│   └── location /                # → index.html, no-cache, SPA 兜底
└── snippets/
    ├── sse.conf                  # 关缓冲 + 长超时, 被 relay location include
    └── ws.conf                   # WebSocket 升级头
```

---

# web

- 不是进程，是一卷静态文件
- 构建产物：`app/web/dist`
- 技术栈不变：React 19 / TypeScript / Rsbuild / Base UI / Tailwind（见 `app/web/AGENTS.md`）
- 包管理器：bun

## 4. 拆分后消失的东西

今天的 embed 契约（见根 `AGENTS.md` 的 "Frontend embed contract" 一节）：
`main.go` 用 `//go:embed web/dist`，而 `go:embed` 不能引用父目录，
所以必须把 `app/web/dist` 拷到 `app/api/web/dist` 再编译 Go。

拆分后整条链路删除：
- `main.go:42-46` — `//go:embed web/dist` + `//go:embed web/dist/index.html`
- `router/web-router.go` — 整个文件（38 行），含 `WebAssets` 结构体、`static.Serve`、`NoRoute` 兜底
- `main.go:243-261` — `InjectUmamiAnalytics`，对 `indexPage` 做 `bytes.ReplaceAll`，
  把 `<!--umami-->` 占位符替换成 script 标签（读 `UMAMI_WEBSITE_ID` / `UMAMI_SCRIPT_URL` 环境变量）
- `main.go:263-284` — `InjectGoogleAnalytics`，同样的字节替换模式
- `make build-web` 里的拷贝步骤
- `.gitignore` 中 `app/api/web/dist` 那条

**分析脚本注入改到构建期**：Rsbuild 的 `html.tags` 或环境变量注入，
或者 nginx 的 `sub_filter`。不要再在运行时改字节。

## 5. 前端功能模块（24 个 feature）

`app/web/src/features/` 下每个目录对应一块管理台功能，与 control 的路由分组一一对应：

- `auth` — 登录、注册、OAuth 回调、2FA、passkey → `/api/user/*`
- `users` — 用户管理 → `/api/user/*`（admin）
- `keys` — API 令牌 → `/api/token/*`
- `channels` — 渠道管理 → `/api/channel/*`（40 条）
- `models` — 模型元数据 → `/api/models/*`
- `pricing` — 定价 → `/api/pricing`、`/api/ratio_config`
- `wallet` — 钱包充值 → `/api/user/topup/*`
- `subscriptions` — 订阅 → `/api/subscription/*`
- `redemption-codes` — 兑换码 → `/api/redemption/*`
- `usage-logs` — 日志查询 → `/api/log/*`
- `dashboard` — 用量看板 → `/api/data/*`
- `rankings` — 排行榜 → `/api/rankings`
- `performance-metrics` — 性能指标 → `/api/perf-metrics/*`
- `system-info` — 节点列表 → `/api/system-info/*`
- `system-settings` — 系统配置 → `/api/option/*`
- `proxy` — 代理节点 → `/api/proxy/*`（15 条）
- `playground` — 在线试用 → `/pg/chat/completions`（**这条打 gateway，不是 control**）
- `chat` — 聊天界面 → 同上
- `profile` — 个人设置 → `/api/user/self/*`
- `setup` — 首次安装 → `/api/setup`
- `home`、`about`、`legal`、`errors` — 静态页

## 6. i18n

- 7 种语言：`app/web/src/i18n/locales/` — `en.json`、`zh.json`、`zh-TW.json`、`fr.json`、`ru.json`、`ja.json`、`vi.json`
- 库：i18next + react-i18next + i18next-browser-languagedetector
- 平铺 JSON，key 就是英文原文
- 工具：`bun run i18n:sync`
- **后端也有 i18n**：`app/api/i18n/`（go-i18n，en/zh），拆分后 gateway 与 control 各自带一份

## 7. 目录结构

```
web/                              # 位置不变, 只是不再被 go:embed
├── package.json                  # bun
├── rsbuild.config.ts             # devProxy 指向 gateway:8080 与 control:8081（今天只指一个）
├── src/
│   ├── main.tsx
│   ├── routes/                   # TanStack Router
│   ├── routeTree.gen.ts
│   ├── features/                 # 24 个功能模块, 见 §5
│   ├── components/
│   ├── hooks/
│   ├── stores/
│   ├── context/
│   ├── lib/
│   ├── config/
│   ├── i18n/locales/             # 7 种语言
│   ├── styles/
│   └── assets/
└── dist/                         # 构建产物 → 直接给 nginx, 不再拷进 Go 二进制
```

## 8. 开发期注意

今天 `rsbuild.config.ts:71` 的 `devProxy` 只需指向一个后端（单体）。
拆分后要指两个：`/api` → control，`/v1` 等 → gateway。
这是拆分对前端唯一的实质影响，业务代码不用改（前端调用的都是相对路径）。
