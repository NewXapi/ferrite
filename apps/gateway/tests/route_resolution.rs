//! P0 路由解析场景登记（骨架，PR #158；规格：todo/gateway-resilience.md P0）。
//!
//! 现状：无头 gateway 冒烟 `POST /v1/chat/completions`（model=mock-gpt、token
//! group=default、channel `models=["mock-gpt"]`）→ 404 `no_route`，未定位。
//! 本文件把 P0 诊断的两个端到端场景钉成测试形状（零外网、CI 常驻可跑），
//! 实现落地时替换 `todo!()` 体。接线形状与 #151 冒烟同款：
//! 临时目录 TOML → `GatewayConfig::load` → `gateway::build_app`（tests/smoke.rs
//! 的进程版 + tests/config_wiring.rs 的 load_toml 版，这里取进程内端口绑定版）。

/// P0-1：配置齐全的最小全链路必须 200，钉死「快照里明明有 route 却 404」。
///
/// 实现要点（TODO(#158)）：
/// 1. 起本地 mock 上游：`std::net::TcpListener` 绑 `127.0.0.1:0`，后台线程
///    accept 并回固定 `chat.completion` JSON（Content-Type: application/json），
///    记录是否收到过请求（= 回包确实经代理节点/上游走过，非缓存幻觉）。零外网。
/// 2. 临时目录写 `config.toml`：`[[keys]]`（token group=default）、
///    `[[channels]]`（models=["mock-gpt"]、base_url 指向上一步端口）、
///    `[[proxy_nodes]]` 三段，形态同 #151 冒烟；`GatewayConfig::load` 真实路径。
/// 3. `gateway::build_app(&cfg)` → `tokio::net::TcpListener` 随机端口 serve →
///    带 Authorization 发 `POST /v1/chat/completions`（model=mock-gpt）→
///    断言 200，且 mock 上游收到请求（对齐 #151 冒烟）。
///    404 复现时按 P0 诊断步骤给 `Dispatcher::select` 加 debug 日志定位
///    （group 提取错 / 快照空 / `gate::model` 未提升 `ctx.requested_model`）。
#[tokio::test]
#[ignore = "TODO(#158): P0 诊断，骨架未实现"]
async fn p0_mock_upstream_chat_completion_resolves_200() {
    todo!()
}

/// P0-2（P1-B 落地后的回归锚点）：双渠道同模型，渠道相关 4xx 降层到第二渠道。
///
/// 实现要点（TODO(#158)，依赖 P1-B `FatalButSwitchable`）：
/// 1. 两个本地 mock 上游：A 对 `POST /v1/chat/completions` 回 404（渠道相关
///    4xx——换渠道可能成立），B 回固定 chat.completion JSON 200；
/// 2. `[[channels]]` 两段各指其一、priority 让 A 先被选中、models 同 mock-gpt；
/// 3. build_app → 请求 → 断言：客户端最终 200（B 的回包）、A 与 B 各收到一次
///    请求（证明发生了降层换候选，而非 A 根本不可达），且 A 的健康不被
///    该类失败污染（Neutral 语义）。400/413/422 请求相关 4xx 的单候选透传
///    形状另见 dispatch/tests/failure_scope.rs。
#[tokio::test]
#[ignore = "TODO(#158): P0 诊断，骨架未实现"]
async fn p0_channel_scoped_404_downgrades_to_second_channel() {
    todo!()
}
