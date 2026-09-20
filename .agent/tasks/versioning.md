# 版本口径（ferrite）

多组件 monorepo，**版本以 git tag 为载体**（组件前缀），Cargo workspace 版本恒 0.1.0 不动。

三段各来源不同：

- **major = 用户确认**（breaking 由用户拍板，当前 0）
- **minor = 该组件用户可感知功能域数**（逐 crate 代码清点，排除管道，可复现）
- **patch = 该组件路径 fix commit 累计数**（pathspec 过滤，机数）

## 各组件 tag 格式与复现命令

```
<component>-v<MAJOR>.<MINOR>.<PATCH>

# patch（组件路径 fix 数）
git log refs/heads/main --no-merges --format="%s" -- <组件路径...> | grep -cE "^fix"
```

## gateway 组件

- 路径：`apps/gateway` + `crates/gateway/*`
- 功能域（minor = 9）：

| 功能域 | crate |
|---|---|
| 路由选型 + 重试 + 限流 + 健康表 | crates/gateway/dispatch |
| 上游转发 + 出口池 + 流式管道 | crates/gateway/forward |
| 访问控制过滤（auth/state/quota/ratelimit/graylist/model/concurrency） | crates/gateway/gate |
| 计量（token 估算 + 定价 + 结算） | crates/gateway/metering |
| 阶段编排 + axum 路由 | crates/gateway/pipeline |
| 厂商协议适配 + SSE 帧扫描 | crates/gateway/protocol-bridge |
| 代理节点池 + 主动探测 + SSRF 防护 + 分享链接 | crates/gateway/proxy |
| 过滤词扫描 | crates/gateway/security |
| 网关数据面组装 / 启动 + 可观测 | apps/gateway |

- **patch = 11**（gateway 路径 fix，`-- apps/gateway crates/gateway`）

## 快照（gateway-v0.9.11）

- major = 0（无 breaking）
- minor = 9（上表 9 项功能域）
- patch = 11（gateway 路径 fix 累计）

新增功能域 +1 / 移除 -1；patch 重跑 pathspec 命令。旧 tag（gateway-v0.41.11/0.42.12）为 feat/fix 提交计数旧口径，保留作历史，不再用。
