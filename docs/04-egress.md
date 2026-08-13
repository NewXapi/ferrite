# egress — 出网代理

- 镜像：官方 `ghcr.io/sagernet/sing-box`（**不用 Rust 重写**）
- 监听：`:1080` SOCKS5（仅集群内可达）
- 副本：1/节点（k8s DaemonSet）
- 替换掉的 Go 代码：`service/singbox_dialer.go`（393 行）+ `service/singbox_registry.go` + `singbox_registry_utls.go` + `singbox_registry_wg.go` + `singbox_registry_wg_stub.go` + `singbox_registry_utls_stub.go`
- 配置来源：control 渲染（见 `07-inter-app-dataflow.md` F9）

---

## 1. 为什么必须独立成进程

### 1.1 今天是进程内跑 sing-box

- 构建 — `service/singbox_dialer.go:29` `BuildSingBoxDialer(outboundJSON)`，内部 `box.New` + `Start`
- 缓存 — `service/singbox_dialer.go:337` `var globalSingBoxDialer singBoxDialerCache`（`mu` + `fingerprint` + `dialer`）
- 取用 — `service/singbox_dialer.go:339` `getSingBoxDialer()`：按指纹比对，不一致就重建并关掉旧的
- 注入 — `service/http_client.go:286-296` 把 `sbDialer.DialContext` 塞进 `http.Transport.DialContext`
- 关闭 — `service/singbox_dialer.go:377` `CloseSingBoxDialer`，在 `main.go:78` defer

**后果**：每个 gateway 副本都是一个独立的 sing-box 客户端，各自建 TCP/TLS/QUIC 会话、
各自 keepalive、各自 uTLS 指纹。扩到 N 副本就是 N 份隧道。

### 1.2 每请求 2 次多余 DB 查询

- `service/singbox_dialer.go:303` `outboundFingerprint()` — **每次调用**执行
  `model.DB.Where("key = ?", "proxy_config").First(&opt)`，然后对 `outbound` 字段算 SHA256
- `service/proxy_config.go:51` `getGlobalProxyURL()` — 又查一次同一行；
  注释 `:47-50` 明确写了这是故意绕过 `OptionMap` 缓存，"让每个实例在下一个请求就能观察到配置变化"

拆出去后这两处一起消失，gateway 只认一个静态 SOCKS5 地址。

### 1.3 依赖体积

`app/api/go.mod` 里 `github.com/sagernet/sing-box v1.12.12` 是直接依赖，
另有 **15 个 sagernet 间接依赖**：`bbolt`、`fswatch`、`gvisor`、`netlink`、`nftables`、
`sing`、`sing-mux`、`sing-shadowsocks`、`sing-shadowsocks2`、`sing-shadowtls`、
`sing-tun`、`sing-vmess`、`smux`、`wireguard-go`、`ws`。

Rust 侧没有等价物，硬翻译不现实。用官方二进制是唯一务实选择。

---

## 2. 内部功能

### 2.1 outbound 协议（对应 `service/singbox_registry.go:45-58`）

- `direct` — `direct.RegisterOutbound`
- `block` — `block.RegisterOutbound`
- `dns` — `protocolDNS.RegisterOutbound`
- `selector` — `group.RegisterSelector`（分组选择）
- `urltest` — `group.RegisterURLTest`（延迟测速选择）
- `socks` — `socks.RegisterOutbound`
- `http` — `shttp.RegisterOutbound`
- `shadowsocks` — `shadowsocks.RegisterOutbound`
- `vmess` — `vmess.RegisterOutbound`
- `trojan` — `trojan.RegisterOutbound`
- `ssh` — `ssh.RegisterOutbound`
- `shadowtls` — `shadowtls.RegisterOutbound`
- `vless` — `vless.RegisterOutbound`
- `anytls` — `anytls.RegisterOutbound`
- `wireguard` — `service/singbox_registry_wg.go` `registerWireGuard`（outbound + endpoint）

### 2.2 传输层（`service/singbox_registry.go:72-77`）

- TCP / UDP / TLS / HTTPS — `transport.RegisterTCP` / `RegisterUDP` / `RegisterTLS` / `RegisterHTTPS`
- hosts / local DNS transport — `hosts.RegisterTransport` / `local.RegisterTransport`

### 2.3 uTLS 指纹

- `service/singbox_registry_utls.go`（有 build tag 的可选实现，另有 `_stub.go` 兜底）

### 2.4 outbound 配置字段（`service/singbox_dialer.go:95-123` `outboundConfigFields`）

这是 control 渲染配置时必须支持的字段全集：

- 基础 — `type`、`server`、`server_port`
- 凭证 — `uuid`、`username`、`password`、`method`、`flow`、`encryption`
- 网络 — `network`、`packet_encoding`
- 混淆 — `masquerade`、`obfs`、`obfs_password`、`hop_ports`
- TLS — `tls_enabled`、`tls_server_name`
- 传输（扁平写法）— `transport_type`、`transport_path`、`transport_host`、`transport_service_name`
- 传输（嵌套写法）— `transport.{type, path, headers, service_name}`

注意 `service/singbox_dialer.go:311-315` 的注释：控制台把传输头存在 `transport.headers` 下，
如果经由 `service/proxy_config.go` 的 `OutboundConfig` 结构体往返一遍会丢掉（它只有扁平的 `Host`），
所以 `outboundFingerprint` 直接从持久化的 Option 值里取原始 JSON。
**Rust 侧渲染配置时必须保留嵌套 `transport.headers`。**

### 2.5 新增职责：SOCKS5 inbound

今天没有 inbound —— 今天是进程内直接 `DialContext`。
拆出去后 egress 必须开一个本地 SOCKS5 inbound 供 gateway 连接。

---

## 3. 数据流动

### 3.1 今天的路径

```
gateway 进程
  └─ relay/channel/api_request.go:491 GetHttpClientWithProxySettings(info.ChannelSetting.Proxy, ...)
      └─ service/http_client.go:286 IsSingBoxScheme(scheme)?
          └─ service/singbox_dialer.go:339 getSingBoxDialer()
              ├─ :303 outboundFingerprint()  ← 查 DB Option.proxy_config
              └─ :29  BuildSingBoxDialer()   ← box.New + Start（进程内）
                  └─ :73 DialContext()       ← 直接拨号出网
```

### 3.2 拆分后的路径

```
gateway 进程
  └─ upstream/socks5.rs
      └─ reqwest::Proxy::all("socks5h://egress:1080")
          └─ [跨进程] ──→ egress 容器
                            └─ sing-box outbound ──→ 外部 AI 上游

control 进程
  └─ egress_cfg/render.rs
      ├─ 读 Option.proxy_config（PG）
      ├─ 读 proxy_nodes 表（PG）
      └─ 写 /etc/sing-box/config.json + reload 信号 ──→ egress 容器
```

### 3.3 代理方案的三条路径（`common/proxy_url.go:35-36`）

gateway 侧的代理选择逻辑（`service/http_client.go:264-299` `configureProxyTransport`）：

- `http` / `https` — `transport.Proxy = http.ProxyURL(proxyURL)`（`:266-268`）
- `socks5` / `socks5h` — `proxy.FromURL` 拿 `ContextDialer`，塞进 `DialContext`（`:269-284`）
- `sing-box` — `common/proxy_url.go:83` `singBoxProxyScheme = "sing-box"`，走进程内拨号器（`:285-297`）

**拆分后第三条路径消失**，`sing-box://` scheme 改成指向 egress 的 `socks5h://`，
复用已有的第二条路径。这意味着 gateway 侧的代理代码从 3 条分支减到 2 条。

### 3.4 代理节点与热路径的关系

- `model/proxy_node.go:96` `GetProxyNodesForChannel` — 按 channel → group → all 三级取节点
- **今天唯一调用点在 `controller/proxy.go`**（管理面测活），**relay 热路径不碰**
- relay 只读 `info.ChannelSetting.Proxy` 字符串（`relay/channel/api_request.go:491`）
- 所以拆 egress **不改变推理逻辑**，只是把拨号从进程内挪到进程外

---

## 4. 目录结构

egress 本身没有 Rust 代码。相关代码分布在两处：

```
deploy/egress/
├── Dockerfile                    # FROM ghcr.io/sagernet/sing-box:latest
├── config.template.json          # inbound: socks5 :1080
│                                 # outbounds: 由 control 渲染填充
└── entrypoint.sh                 # 等待 control 首次下发配置后再启动

bins/control/src/egress_cfg/      # 配置渲染方（详见 02-control.md）
├── node.rs                       # ← model/proxy_node.go + service/proxy_node.go(9符号)
├── parser.rs                     # ← service/proxy_node_parser.go（分享链接解析）
├── probe.rs                      # ← service/proxy_node_probe.go(9符号)
└── render.rs                     # 渲染 15 种 outbound + 嵌套 transport.headers
                                  #   字段全集见 §2.4（← singbox_dialer.go:95-123）

bins/gateway/src/                 # 使用方（详见 01-gateway.md）
└── (crates/upstream/src/socks5.rs)  # reqwest::Proxy::all("socks5h://egress:1080")
```

**删除清单**（拆分完成后）：
- `app/api/service/singbox_dialer.go`（393 行）
- `app/api/service/singbox_registry.go`（79 行）
- `app/api/service/singbox_registry_utls.go` + `_stub.go`
- `app/api/service/singbox_registry_wg.go` + `_stub.go`
- `app/api/service/proxy_config.go:51` `getGlobalProxyURL`（每请求查 DB）
- `app/api/common/proxy_url.go:83-88` 的 `sing-box` scheme 分支
- `go.mod` 中 `sagernet/sing-box` + 15 个间接依赖

---

## 5. 部署形态

- k8s：**DaemonSet**，每个节点一个。gateway 通过 `hostPort` 或节点本地 Service 连本机 egress，
  这样出网流量不跨节点，也让每个 k8s 节点的出口 IP 各自承担一部分流量
- docker-compose：单个 service，gateway 通过 service 名 `egress:1080` 连接
- 配置热重载：写文件 + `SIGHUP`，或走 sing-box 的 Clash API
- **安全**：SOCKS5 inbound 不要暴露到集群外。它是无认证的内网服务，
  暴露出去等于开放代理

## 6. 灰度与回滚

拆 egress 是迁移的第 1 步（见 `08-migration-order.md`），因为：
- 它不碰计费、不碰路由、不碰数据一致性
- 验证方式简单：走代理的渠道测活能过就行
- 回滚只需把渠道的 `Proxy` 字段从 `socks5h://egress:1080` 改回 `sing-box://...`
- 收益立即可见：`go.mod` 少 16 个依赖，每个走代理的请求少 2 次 DB 查询
