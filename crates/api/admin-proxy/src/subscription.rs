//! `subscription` —— 订阅批量导入与分享链接批量导入。
//!
//! 订阅拉取与 YAML 解析**全部委托** [`meow_config::subscription`]：它已处理 HTTP
//! 拉取（带 UA 与体积上限）、`<<: *anchor` merge 键展开、`proxies` 序列提取。
//! 本模块只做 clash proxy map → `proxy_nodes` 行的映射。
//!
//! `SubscriptionData` 的 `proxy_groups` / `rules` 两个字段解析后直接丢弃：我们的
//! 选点语义是「渠道 → 节点集合 + priority」（见 `gateway_proxy::pool`），不需要
//! clash 的 url-test / fallback 组，也不需要域名规则路由。
//!
//! # `clash_proxy_to_url` 是有损映射
//!
//! `proxy_nodes.url` 是分享链接形态的 TEXT，表达力比 clash map 窄：`grpc-opts`、
//! `mux`、`smux`、`ech-opts`、多值 `alpn` 在我们的 URL query 里**没有对应键**。
//!
//! 约定：遇到无法表达的键 → 返回 `Err`，调用方记为 [`ImportFailure`]。宁可导入失败
//! 让用户知道，不要静默丢配置——一个丢了 `grpc-opts` 的节点会拨号成功但走错传输层，
//! 排查成本远高于导入时报错。
//!
//! 真出现大量此类节点时的升级路径：给 `proxy_nodes` 加 `clash_config JSONB` 列，
//! `load_proxy_snapshot` 优先用它，`adapter_for` 加 `adapter_from_clash(map)` 入口。
//! 代价是 `ProxyNode` 不再是唯一真相，多一条装配路径要同步维护。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_yaml::Value as Yaml;

use crate::{ProxyNodeService, ServiceError};

/// 导入请求：订阅 URL 或直接粘贴的文本，二选一。
///
/// `url` 与 `text` 同时为 `None` 或同时有值都是 400（歧义请求不猜意图）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRequest {
    /// 订阅 URL（clash YAML 订阅）。
    #[serde(default)]
    pub url: Option<String>,
    /// 直接粘贴的内容：订阅 YAML 原文，或多行分享链接（取决于调用的端点）。
    #[serde(default)]
    pub text: Option<String>,
    /// 导入的节点统一绑定到这些渠道。空数组是 400——空绑定的节点永远不会被选中。
    pub channel_keys: Vec<String>,
    /// 导入的节点统一优先级。
    #[serde(default)]
    pub priority: i32,
}

/// 单条导入失败。
///
/// `source` 必须是**掩码后**的形态：订阅与分享链接里都带密码 / UUID，
/// 原样回显等于把凭据写进前端和日志。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFailure {
    /// 掩码后的来源标识（`vless://***@host:port`，或订阅里的节点名）。
    pub source: String,
    /// 失败原因：不支持的协议 / 有损映射拒绝 / 校验失败 / DB 冲突。
    pub reason: String,
}

/// 导入结果逐条报告。
///
/// 为什么不只回计数：用户需要知道**哪个**节点**为什么**没进去。
/// 「导入 40 个成功 31 个」这种回复无法排查——剩下 9 个是协议不支持、
/// 渠道名写错、还是有损映射拒绝？
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    /// 成功入库条数。
    pub created: usize,
    /// 跳过条数（同 URL 已存在，非错误）。
    pub skipped: usize,
    /// 失败明细。
    pub failures: Vec<ImportFailure>,
}

/// clash proxy map → 分享链接形态 URL（有损，见模块头）。
///
/// # 参数
/// `proxy` 是 `meow_config::subscription::SubscriptionData::proxies` 的一个元素，
/// 键名遵循 clash/mihomo 约定：`type` / `server` / `port` / `uuid` / `password` /
/// `cipher` / `ws-opts` / `reality-opts` / `client-fingerprint` / `sni` / `flow`。
///
/// # 错误
/// - 缺 `type` / `server` / `port`：无法定位节点
/// - `type` 不在我们的 [`gateway_proxy::ProxyScheme`] 白名单内（如 tuic / wireguard）
/// - 出现 URL query 无法表达的键（`grpc-opts` / `mux` / `smux` / `ech-opts` /
///   多值 `alpn`）：**拒绝而非丢弃**
///
/// # 映射约定
/// 密码字段位置必须与 `gateway_proxy::adapter` 的 auth 语义一致（反向映射）：
/// ss → `ss://cipher:password@`；vless / vmess → `scheme://uuid@`；
/// trojan / hysteria2 / anytls / snell → `scheme://password@`（单段 userinfo）。
/// 搞错这个会导致节点装配后认证失败，#98 已在 trojan 上踩过一次。
pub fn clash_proxy_to_url(proxy: &HashMap<String, Yaml>) -> Result<String, String> {
    let _ = proxy;
    todo!("TODO(#111): clash map → 分享链接 URL；无法无损表达的键返回 Err（见模块头有损映射约定）")
}

/// 导入订阅：拉取 / 解析 → 逐条 [`clash_proxy_to_url`] → 复用
/// [`ProxyNodeService::create`]。
///
/// 复用 `create` 而不新写 INSERT：URL 校验、渠道引用完整性、`updated_at`
/// 都在里面，绕过它等于把三处逻辑抄第二遍。
///
/// # 参数
/// `req.url` 走 [`meow_config::subscription::fetch_subscription`]，
/// `req.text` 走 [`meow_config::subscription::parse_subscription_yaml`]。
///
/// # 错误
/// 整体失败（返回 `Err`）只有两种情况：请求歧义（url/text 都给或都不给）、
/// 订阅拉取或 YAML 解析失败。**单个节点的失败不算整体失败**——进
/// [`ImportReport::failures`]，其余节点继续导入。
pub async fn import_subscription(
    svc: &ProxyNodeService,
    req: &ImportRequest,
) -> Result<ImportReport, ServiceError> {
    let _ = (svc, req);
    todo!("TODO(#111): 订阅导入；单节点失败进 report.failures，不中断整批")
}

/// 导入粘贴的多行分享链接。
///
/// 解析走 `gateway_proxy::sharelink::parse_share_links`（吃 vmess base64-JSON
/// 与 ss legacy 方言），拿到的 `ShareLinkBatch::nodes` 原样用原始行入库
/// （**不要**把 `ProxyNode` 再序列化回 URL——那一圈往返会丢 query 里的未识别键）。
///
/// # 错误
/// 与 [`import_subscription`] 同：整体失败仅限请求歧义；单行失败进 report。
pub async fn import_share_links(
    svc: &ProxyNodeService,
    req: &ImportRequest,
) -> Result<ImportReport, ServiceError> {
    let _ = (svc, req);
    todo!("TODO(#111): 多行分享链接批量导入；用原始行入库而非 ProxyNode 反序列化")
}
