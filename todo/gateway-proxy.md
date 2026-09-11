# gateway proxy 出口代理路线图

> 网关数据面的**按渠道出口代理**能力：每个上游渠道可绑定一组代理节点，
> 模型请求经该节点出网。装配层用 [meow-config](https://crates.io/crates/meow-config) 0.21.2
> （clash/mihomo 配置模型的 Rust 实现），管理面对标 grok2api-sing
> `backend/internal/infra/egress/`。
>
> 工作目录: `.wt/<name>/`（见根 `AGENTS.md`）。域归属: `crates/gateway/proxy`（数据面）
> + `crates/api/admin-proxy`（管理面）。

## 一、现状：已经能用的部分

### 数据链路（已闭环）

```
proxy_nodes 表 (admin-proxy)
   │  load_proxy_snapshot: enabled 行 → ProxyNode
   ▼
ProxySnapshot ──install──▶ ProxyPool  (ArcSwap<HashMap<channel_key, Vec<Arc<ProxyNode>>>>)
                              │  candidates(channel_key) 按 priority 降序
                              ▼
                          ProxyManager::acquire(channel_key) ──▶ Lease { node_id, client }
                              │                                        │
                              │ client_for(node) 按 scheme 分派         │
                              ├─ Direct/Http/Socks5 → reqwest::Client ─┤
                              └─ 7 种协议 → adapter_for(node)          │
                                    │  clash_config() → meow parse_proxy
                                    ▼                                  ▼
                                Arc<dyn ProxyAdapter>          ForwardStage 发上游
                                    │  dial_tcp 内完成协议握手           │
                                    └─ adapter_egress 桥 hyper+rustls ──┘
                                                                       │
                                             feedback(node_id, status, transport_err)
                                                                       ▼
                                                          NodeHealth 指数冷却
```

### 能力清单

| 能力 | 落地位置 | PR |
|---|---|---|
| 7 种协议出口（ss / trojan / vless / vmess / hysteria2 / anytls / snell） | `proxy/src/adapter.rs` `clash_config()` | #97 #102 |
| ws / grpc / h2 / httpupgrade 传输层 + REALITY + uTLS 指纹 | meow-config 吸收（我们只做字段映射） | #100 #101 #102 |
| 按 channel_key 选节点：priority 分层 + 层内 least-inflight + 并列随机 | `manager.rs::acquire` | #74 |
| per-node client 缓存 `(node_id, fingerprint)` 双缓存（reqwest / adapter 各一份） | `manager.rs` `client_cache` / `connector_cache` | #74 #84 |
| 失败自动冷却：指数退避 `min(10min, 30s << min(n-1,4))`，401/429 不冷却 | `manager.rs::feedback` | #74 |
| inflight 计数与 `Lease::drop` 自动释放 | `manager.rs` + `Lease` | #74 |
| 配置错误一律 warn + 回落直连（不 panic 网关，不返 502） | `manager.rs::client_for` / `adapter.rs::adapter_for` | #84 |
| SSRF 防护（IP 字面量 + DNS 解析结果双重校验） | `proxy/src/ssrf.rs` | 早期 |
| `proxy_nodes` 表 + CRUD + 掩码回传 + 渠道引用完整性 + probe 端点 | `crates/api/admin-proxy/src/lib.rs`（DDL 已迁 `db/migrations/0005_ops_billing_proxy.sql`） | #106 |
| CRUD 变更后原地热更新（`reload_into` → `install`），apps/api 启动装载 | `admin-proxy` + `apps/api/src/lib.rs` | #108 |
| 真节点拨号验证（env 门控，三层降级） | `forward/tests/protocol_live.rs` | #98 |
| 管理台「出口代理节点」编辑器（前端态 + URL 校验） | `web/admin-page-admin/src/system.rs` | #96 |

### 关键语义（改动前必读）

- **节点 URL 就是分享链接格式**：`vless://uuid@host:port?flow=&sni=&pbk=&sid=&type=ws&path=`。
  `pbk` 是 43 字符 base64url（**不是** 64 hex——#98 真节点验证纠正的第一个 bug）。
- **密码字段位置**：trojan / hysteria2 / anytls / snell 的密码在 `auth.user`（`trojan://密码@host`
  的 userinfo 只有一段），**不在** `auth.pass`（#98 纠正的第二个 bug，#103 补的回归测试）。
  ss 是 `ss://cipher:pass@` 两段；vless/vmess 的 `auth.user` 是 UUID。
- **`node_id == 0` 是直连哨兵**：无节点 / 全部冷却 / 装配失败都回落到它，`feedback` 对它无操作。
- **ProxyPool 是最终一致**：`install` 原子换指针，持旧 `Arc` 的读者看旧快照到下次 `load()`。
  代理选择容忍这点滞后（最坏结果是这一个请求走了旧出口），换读路径零锁。
- **硬拒 vs 回落的边界**（ADR-0002 Class A）：未知/废弃 VLESS `flow`、未知 VMess cipher、
  `zero` cipher → 装配期硬错（配置写错了要让人知道）；节点运行期失败 → 冷却 + 回落直连。
- **本地真拨验证**：
  `FERRITE_PROXY_VLESS='vless://...' cargo test -p forward --test protocol_live`
  （缺 env 跳过；`FERRITE_PROXY_LIVE_STRICT=1` 强制失败为红）。

### 现在还差什么

管理面能增删改节点，数据面能选节点出网，但**节点从哪来**、**节点还活着吗**这两件事没有答案：

1. 用户手上是机场给的**分享链接**（`vmess://` 是 base64 包的 JSON，`ss://` 有 legacy
   base64 方言），当前 `ProxyNode::parse_url` 只吃标准 URL 形态 → 用户得手工翻译。
2. 用户手上是**订阅 URL**（一条链接几十个节点），当前只能一个个粘。
3. 节点是否可用只有**被动**信号（请求失败才冷却）。没有主动探测，管理台看不到延迟，
   坏节点要等真实流量踩一次才下线。

M2 解决 1、2（入口），M3 解决 3（可观测）。

## 二、M2 — 节点入口

### M2-A 分享链接解析（`crates/gateway/proxy/src/sharelink.rs`）

**问题**：`parse_url` 是标准 URL 解析器，吃不下两种主流方言：

| 方言 | 形态 | 现状 |
|---|---|---|
| VMess v2rayN | `vmess://` + base64(JSON)，字段 `v/ps/add/port/id/aid/scy/net/type/host/path/tls/sni/fp/alpn` | 完全不支持（`Url::parse` 后 host 是 base64 串） |
| SS legacy | `ss://` + base64(`method:password`) `@host:port#备注` | 不支持（userinfo 是整段 base64） |
| SS SIP002 | `ss://` + base64(`method:password`) 或明文 `@host:port` | 部分支持（明文 userinfo 走通） |

**交付**：
- `parse_share_link(link) -> Result<ProxyNode, ParseError>` 统一入口，按 scheme 分派到方言解析器，
  其余 scheme 直接委托现有 `parse_url`（**不重复实现**）。
- `parse_share_links(text) -> ShareLinkBatch` 多行批量，逐行成功/失败分别归集
  （一行坏链接不能让整批失败）。
- 结构体 `VmessShareLink`（v2rayN JSON 的 serde 映射）+ `ShareLinkBatch`。

**验收**：`cargo test -p gateway-proxy --test sharelink`，覆盖 vmess base64 往返、
ss legacy、ss SIP002、坏 base64、缺字段、批量里混坏行。

**非目标**：tuic / ssh / wireguard / shadowtls —— meow-config 解析器有，但我们
`ProxyScheme` 没这几个变体，加了得同步 `clash_config` + `to_reqwest_proxy` + 真节点验证。
等有真实需求再开（届时按 #97 加 Hysteria2 的同一套改法）。

### M2-B 订阅批量导入（`crates/api/admin-proxy/src/subscription.rs`）

**已有的**：`meow_config::subscription::{fetch_subscription, parse_subscription_yaml}`
返回 `SubscriptionData { proxies: Vec<HashMap<String, Yaml>>, proxy_groups, rules }`
—— clash YAML 拉取 + 解析 + `<<: *anchor` merge 展开全都现成。

**要做的**：把 clash proxy map 落进 `proxy_nodes` 表。

**⚠ 待定的设计决策（实现者必须先定这个）**：`proxy_nodes.url` 是 TEXT，存的是分享链接
形态。clash map → URL 是**有损**的：`grpc-opts`、`mux`、`smux`、`ech-opts`、多值 `alpn`
在我们的 URL query 里没有表达。两条路：

| 方案 | 改动 | 代价 |
|---|---|---|
| **A. 有损 URL 化**（骨架当前签名） | 只加 `clash_proxy_to_url()`，schema 不变 | 带上述字段的节点导入即丢配置，用户看不出来 |
| **B. 加 `clash_config JSONB` 列** | DDL 加列；`load_proxy_snapshot` 优先用它；`adapter_for` 加 `adapter_from_clash(map)` 入口 | 多一条装配路径，`ProxyNode` 不再是唯一真相 |

建议：**先 A，且 `clash_proxy_to_url` 遇到无法表达的键就把该节点记为 failure**（宁可导入失败
让用户知道，不要静默丢配置）。真出现大量此类节点再升 B。骨架的 rustdoc 里记了这条。

**交付**：
- `import_subscription(pool, req) -> Result<ImportReport, ServiceError>`：拉订阅 → 逐条
  `clash_proxy_to_url` → 复用 `ProxyNodeService::create`（校验/引用完整性/热更新全都白拿）。
- `import_share_links(pool, req) -> Result<ImportReport, ServiceError>`：粘贴多行分享链接批量入库
  （吃 M2-A 的 `parse_share_links`）。
- `POST /api/proxy_nodes/subscription`、`POST /api/proxy_nodes/batch` 两个端点。
- `ImportReport { created, skipped, failures: Vec<ImportFailure> }` —— **逐条报告，不要
  只回一个数字**：用户需要知道哪个节点为什么没进去。

**验收**：`cargo test -p admin-proxy --test subscription`（clash map → URL 映射的纯函数
测试不需要 DB；入库路径按 `#[ignore]` + `DATABASE_URL` 惯例）。

### M2-C 前端接入

管理台「出口代理节点」面板加「粘贴分享链接 / 订阅 URL」输入框，调 M2-B 两个端点，
按 `ImportReport` 逐条展示结果。按 `specs/ui/system.yaml` 补 `data-testid`
（约定见 `.agent/skills/ui-validation/SKILL.md`）。

## 三、M3 — 主动探测与可观测

### 复用现成的：meow 每个 adapter 自带健康表

`meow_common::ProxyAdapter::health() -> &ProxyHealth`，`ProxyHealth` 有
`alive()` / `last_delay()` / `delay_history()`（滚动 10 条）/ `record_delay(u16)`。
**不要自己再造一套延迟历史**——探测拨号后调 `record_delay` 即可。

### M3-A 主动探测（`crates/gateway/proxy/src/probe.rs`）

- `probe_node(node, target, timeout) -> ProbeResult`：装配 adapter → `dial_tcp` 计时 →
  写回 `health().record_delay()`。**只拨 TCP，不发 HTTP 请求**（够判活，且不消耗上游额度）。
- `ProxyManager::probe_all(target, timeout) -> Vec<ProbeResult>`：并发探测当前快照全部节点。
- `ProxyManager::node_stats() -> Vec<NodeStats>`：把进程内私有状态（inflight / failure_count /
  cooldown 剩余 / last_delay）导出成可序列化视图 —— 管理台的数据源。

`NodeStats` 是**新的公开出口**：`inflight` / `health` 现在是私有字段，管理台看不到任何运行时
状态。这个方法一加，`GET /api/proxy_nodes/report` 就有内容可返。

### M3-B 定时探测与报告端点

- `admin-proxy`: `GET /api/proxy_nodes/report` → `node_stats()` + 节点元信息 join。
- `POST /api/proxy_nodes/{key}/probe` 升级：当前只做**装配**校验（`probe_build`，不拨号），
  改为可选真拨（query `?dial=1`）。保留装配校验模式——它不产生网络流量，是填表时的即时反馈。
- 定时器：`apps/api` 起一个 `tokio::spawn` 周期 `probe_all`（间隔走 options 表，缺省 5min）。
  **默认关闭**：主动探测会给机场带流量，得用户显式开。

**验收**：`probe.rs` 的纯逻辑（超时判定 / ProbeResult 归类）单测；真拨按 `protocol_live.rs`
的 env 门控惯例。

## 四、M4 / M5 — 明确不做与低优先级

### 已经做完了，不要重复造

盘点时发现下面三项**已在 `manager.rs` 实现**（早期路线图误标为待做）：

| 曾以为要做 | 实际位置 |
|---|---|
| 自动熔断 / 连续失败冷却 | `manager.rs::feedback` 指数退避，401/429 豁免 |
| per-node 连接池 | `manager.rs` `client_cache` + `connector_cache`，key `(node_id, fingerprint)` |
| least-inflight 选点 | `manager.rs::acquire` priority 层内 least-inflight + 并列随机 |

### M5 低优先级

- `VlessOpts` → `NodeOpts` 改名：它现在承载 vless/vmess/hysteria2/anytls/snell 五种协议的
  opts，名字骗人。breaking，随大版本做。
- meow-config 依赖瘦身：`maxminddb` / `meow-rules` 我们用不到，能 feature-gate 掉。
- `utls` feature 的 CI 编译验证（需要 `cmake`，README 已写前提）。

### 明确不做

- **sing-box / 任何 sidecar 进程**：#84 已裁掉（5319 行手抄 proto + 子进程管理换成
  meow-config in-process）。不要回头。
- **UDP 出口**：`dial_udp` 在 trait 里，但网关只转发 HTTP 模型请求，用不到。
- **代理组 / 规则路由**（clash 的 `proxy-groups` / `rules`）：我们的选点语义是
  「渠道 → 节点集合 + priority」，不需要 clash 的 url-test/fallback 组。
  订阅里的 `proxy_groups` / `rules` 字段解析后直接丢弃。
